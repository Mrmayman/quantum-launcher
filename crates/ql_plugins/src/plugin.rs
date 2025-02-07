use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc},
};

use mlua::{FromLua, Function, Lua, StdLib, Table, UserData, UserDataMethods, Value, Variadic};
use ql_core::{
    err, file_utils, get_java_binary, info, json::JavaVersion, pt, GenericProgress, IntoIoError,
    IoError,
};
use tokio::runtime::Runtime;

use crate::{json::PluginJson, PluginError};

pub struct Plugin {
    lua: Lua,
    code: String,
    runtime: Arc<Runtime>,
}

impl Plugin {
    pub fn from_code(code: String) -> Result<Self, PluginError> {
        let lua = create_lua()?;

        let globals = lua.globals();

        fn_logging(&globals, &lua)?;

        let plugin = Self {
            lua,
            code,
            runtime: Arc::new(Runtime::new().map_err(PluginError::TokioRuntime)?),
        };

        let runtime = plugin.runtime.clone();
        globals.set(
            "qlJavaExec",
            plugin.lua.create_function(move |_, args| {
                let runtime = runtime.clone();
                l_install_java(runtime, args)
            })?,
        )?;

        Ok(plugin)
    }

    pub fn new(name: &str, version: Option<&str>) -> Result<Self, PluginError> {
        let lua = create_lua()?;

        let globals = lua.globals();

        fn_logging(&globals, &lua)?;

        let plugins_top_dir = file_utils::get_launcher_dir_s()?.join("plugins");
        std::fs::create_dir_all(&plugins_top_dir).path(&plugins_top_dir)?;
        let plugins = std::fs::read_dir(&plugins_top_dir).path(&plugins_top_dir)?;

        let mut plugins_map = HashMap::new();

        for plugin in plugins {
            let plugin = plugin.map_err(|n| IoError::ReadDir {
                error: n,
                parent: plugins_top_dir.clone(),
            })?;
            let json_path = plugin.path().join("index.json");
            let json = std::fs::read_to_string(&json_path).path(json_path)?;
            let json: PluginJson = serde_json::from_str(&json)?;
            let file_name = plugin.file_name();
            plugins_map.insert(file_name.to_str().unwrap().to_owned(), json);
        }

        let Some((name, json)) = plugins_map.iter().find(|(_, j)| {
            println!(
                "{name} == {} && {version:?} == {}",
                j.details.name, j.details.version
            );
            (j.details.name == name) && (version.map(|v| j.details.version == v).unwrap_or(true))
        }) else {
            return Err(PluginError::PluginNotFound(
                name.to_owned(),
                version.map(|n| n.to_owned()),
            ));
        };

        let plugin_root_dir = plugins_top_dir.join(name);
        let main_path = plugin_root_dir.join(&json.main_file.filename);
        let main_code = std::fs::read_to_string(&main_path).path(main_path)?;

        let plugin = Self {
            lua,
            code: main_code,
            runtime: Arc::new(Runtime::new().map_err(PluginError::TokioRuntime)?),
        };

        let runtime = plugin.runtime.clone();
        globals.set(
            "qlJavaExec",
            plugin.lua.create_function(move |_, args| {
                let runtime = runtime.clone();
                l_install_java(runtime, args)
            })?,
        )?;

        for file in &json.files {
            let lua_file = plugin_root_dir.join(&file.filename);
            let lua_file = std::fs::read_to_string(&lua_file).path(lua_file)?;

            let result: Table = plugin.lua.load(lua_file).eval()?;
            globals.set(file.import.clone(), result)?;
        }

        // TODO: deal with dependencies

        Ok(plugin)
    }

    pub fn set_generic_progress(
        &self,
        sender: Sender<GenericProgress>,
        name: &str,
    ) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        globals.set(name, LuaGenericProgress(Arc::new(sender)))?;
        Ok(())
    }

    pub fn init(&self) -> Result<(), PluginError> {
        self.lua.load(&self.code).exec()?;
        Ok(())
    }

    pub fn call_fn(&self, name: &str) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        let func: Function = globals.get(name)?;
        func.call::<()>(())?;
        Ok(())
    }
}

fn create_lua() -> Result<Lua, PluginError> {
    let lua = Lua::new();
    lua.load_std_libs(
        StdLib::BIT
            | StdLib::BUFFER
            | StdLib::COROUTINE
            | StdLib::MATH
            | StdLib::PACKAGE
            | StdLib::STRING
            | StdLib::TABLE
            | StdLib::UTF8
            | StdLib::VECTOR,
    )?;
    Ok(lua)
}

fn fn_logging(globals: &mlua::Table, lua: &Lua) -> Result<(), PluginError> {
    globals.set(
        "qlLogInfo",
        lua.create_function(|_, args: Variadic<Value>| {
            let mut msg = String::new();
            for arg in args {
                msg.push_str(&format!("{} ", arg.to_string()?));
            }
            info!("{msg}");
            Ok(())
        })?,
    )?;
    globals.set(
        "qlLogError",
        lua.create_function(|_, args: Variadic<Value>| {
            let mut msg = String::new();
            for arg in args {
                msg.push_str(&format!("{} ", arg.to_string()?));
            }
            err!("{msg}");
            Ok(())
        })?,
    )?;
    let fn_log_pt = lua.create_function(|_, args: Variadic<Value>| {
        let mut msg = String::new();
        for arg in args {
            msg.push_str(&format!("{} ", arg.to_string()?));
        }
        pt!("{msg}");
        Ok(())
    })?;
    globals.set("qlLogPt", fn_log_pt.clone())?;
    globals.set("print", fn_log_pt)?;

    Ok(())
}

#[derive(Debug)]
pub struct StrErr(String);

impl std::fmt::Display for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StrErr {}

#[derive(Clone)]
struct LuaGenericProgress(Arc<Sender<GenericProgress>>);

impl FromLua for LuaGenericProgress {
    fn from_lua(value: Value, _: &Lua) -> Result<Self, mlua::Error> {
        match value {
            Value::UserData(ud) => {
                let java_progress = ud.borrow::<Self>()?;
                Ok((*java_progress).clone())
            }
            _ => unreachable!(),
        }
    }
}

impl UserData for LuaGenericProgress {
    fn add_methods<M: UserDataMethods<Self>>(_: &mut M) {
        // methods.add_method("magnitude", |_, vec, ()| {
        //     let mag_squared = vec.0 * vec.0 + vec.1 * vec.1;
        //     Ok(mag_squared.sqrt())
        // });

        // methods.add_meta_function(MetaMethod::Add, |_, (vec1, vec2): (Vec2, Vec2)| {
        //     Ok(Vec2(vec1.0 + vec2.0, vec1.1 + vec2.1))
        // });
    }
}

fn l_install_java(
    runtime: Arc<Runtime>,
    args: (String, i32, Option<LuaGenericProgress>),
) -> Result<(), mlua::Error> {
    let version = match args.1 {
        8 => JavaVersion::Java8,
        16 => JavaVersion::Java16,
        17 => JavaVersion::Java17Gamma,
        170 => JavaVersion::Java17Beta,
        171 => JavaVersion::Java17GammaSnapshot,
        21 => JavaVersion::Java21,
        ver => {
            return Err(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                "Could not determine valid java version: {ver}. Valid inputs"
            )))))
        }
    };
    let arc = args.2.map(|n| n.0.clone());
    runtime
        .block_on(get_java_binary(
            version,
            &args.0,
            if let Some(sender) = &arc {
                Some(&sender)
            } else {
                None
            },
        ))
        .map_err(|err| mlua::Error::ExternalError(Arc::new(err)))?;
    Ok(())
}
