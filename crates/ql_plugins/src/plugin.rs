use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Arc},
};

use mlua::{Function, Lua, StdLib, Value, Variadic};
use ql_core::{
    err, file_utils, get_java_binary, info, json::JavaVersion, pt, GenericProgress,
    InstanceSelection, IntoIoError, IoError,
};
use tokio::runtime::Runtime;

use crate::{
    json::{PluginJson, PluginPermission},
    passed_types::{LuaGenericProgress, SelectedInstance},
    PluginError,
};

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

        plugin.fn_java(&globals)?;

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

        if json.permissions.contains(&PluginPermission::Java) {
            plugin.fn_java(&globals)?;
        }

        for file in &json.files {
            file.load(&plugin_root_dir, &globals, &plugin.lua)?;
        }

        // TODO: semver loose check
        plugin.resolve_deps(json, &plugins_map, &plugins_top_dir, &globals)?;

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

    pub fn set_selected_instance(
        &self,
        instance: InstanceSelection,
        name: &str,
    ) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        globals.set(
            name,
            SelectedInstance {
                instance,
                path: PathBuf::new(),
                dot_mc: false,
            },
        )?;
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

    fn fn_java(&self, globals: &mlua::Table) -> Result<(), PluginError> {
        let runtime = self.runtime.clone();

        let func = self.lua.create_function(
            move |_,
                  (name, version, progress, args): (
                String,
                i32,
                Option<LuaGenericProgress>,
                Vec<Value>,
            )| {
                let runtime = runtime.clone();
                let version = match version {
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
                let arc = progress.map(|n| n.0.clone());
                let bin_path = runtime
                    .block_on(get_java_binary(
                        version,
                        &name,
                        if let Some(sender) = &arc {
                            Some(&sender)
                        } else {
                            None
                        },
                    ))
                    .map_err(|err| mlua::Error::ExternalError(Arc::new(err)))?;

                let args: Result<Vec<String>, mlua::Error> =
                    args.into_iter().map(|n| n.to_string()).collect();

                let command = match std::process::Command::new(&bin_path).args(&args?).output() {
                    Ok(n) => n,
                    Err(err) => {
                        return Err(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                            "Could not execute command {bin_path:?}: {err}",
                        )))));
                    }
                };

                if !command.status.success() {
                    let stdout = std::str::from_utf8(&command.stdout)?;
                    let stderr = std::str::from_utf8(&command.stderr)?;
                    return Err(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                        "Java: {name} command failed\n\nStdout: {stdout}\n\nStderr: {stderr}",
                    )))));
                }

                Ok(())
            },
        )?;

        globals.set("qlJavaExec", func)?;
        Ok(())
    }

    fn resolve_deps(
        &self,
        json: &PluginJson,
        plugins_map: &HashMap<String, PluginJson>,
        plugins_top_dir: &Path,
        globals: &mlua::Table,
    ) -> Result<(), PluginError> {
        let Some(deps) = &json.dependencies else {
            return Ok(());
        };
        for (name, dep) in deps {
            let Some((name, json)) = plugins_map
                .iter()
                .find(|(_, j)| (&j.details.name == name) && (j.details.version == dep.version))
            else {
                return Err(PluginError::PluginNotFound(
                    name.to_owned(),
                    Some(dep.version.clone()),
                ));
            };

            let plugin_root_dir = plugins_top_dir.join(name);
            for file in &json.files {
                file.load(&plugin_root_dir, globals, &self.lua)?;
            }
            json.main_file.load(&plugin_root_dir, globals, &self.lua)?;

            self.resolve_deps(json, plugins_map, plugins_top_dir, globals)?;
        }
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

pub fn err_to_lua(err: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::ExternalError(Arc::new(StrErr(format!("{err}"))))
}

#[derive(Debug)]
pub struct StrErr(String);

impl std::fmt::Display for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StrErr {}
