use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Arc},
};

use mlua::{Function, Lua, StdLib, Table, Value, Variadic};
use ql_core::{
    err, file_utils, info, pt, GenericProgress, InstanceSelection, IntoIoError, IoError,
    CLASSPATH_SEPARATOR,
};
use ql_java_handler::{get_java_binary, JavaVersion};
use tokio::runtime::Runtime;

use crate::{
    json::{PluginJson, PluginPermission},
    passed_types::{LuaGenericProgress, SelectedInstance},
    PluginError,
};
use lazy_static::lazy_static;

lazy_static! {
    #[allow(clippy::ref_option)]
    pub static ref RUNTIME: Arc<Runtime> =
        Arc::new(Runtime::new().unwrap());
}

pub struct Plugin {
    lua: Lua,
    code: String,
    mod_map: HashMap<String, String>,
}

impl Plugin {
    pub fn from_code(code: String) -> Result<Self, PluginError> {
        let lua = create_lua()?;

        let globals = lua.globals();

        fn_logging(&globals, &lua)?;

        let plugin = Self {
            lua,
            code,
            mod_map: HashMap::new(),
        };

        plugin.fn_java(&globals)?;

        Ok(plugin)
    }

    pub fn new(name: &str, version: Option<&str>) -> Result<Self, PluginError> {
        let lua = create_lua()?;

        let globals = lua.globals();

        fn_logging(&globals, &lua)?;

        let (plugins_top_dir, plugins_map) = read_plugins()?;

        let Some((name, json)) = plugins_map.iter().find(|(_, j)| {
            (j.details.name == name) && (version.is_none_or(|v| j.details.version == v))
        }) else {
            return Err(PluginError::PluginNotFound(
                name.to_owned(),
                version.map(std::borrow::ToOwned::to_owned),
            ));
        };

        let plugin_root_dir = plugins_top_dir.join(name);
        let main_path = plugin_root_dir.join(&json.main_file.filename);
        let main_code = std::fs::read_to_string(&main_path).path(main_path)?;

        let mut plugin = Self {
            lua,
            code: main_code,
            mod_map: HashMap::new(),
        };

        if json.permissions.contains(&PluginPermission::Java) {
            plugin.fn_java(&globals)?;
        }
        if let Some(PluginPermission::Request { whitelist }) = json
            .permissions
            .iter()
            .find(|n| matches!(n, PluginPermission::Request { .. }))
        {
            plugin.fn_download(whitelist.to_owned(), &globals)?;
        }
        plugin.fn_file_pick(&globals)?;

        for file in &json.files {
            file.load(&plugin_root_dir, &mut plugin.mod_map)?;
        }

        plugin.resolve_include_file(json, &plugin_root_dir, &globals)?;

        // TODO: semver loose check
        plugin.resolve_deps(json, &plugins_map, &plugins_top_dir, &globals)?;

        let table = plugin.lua.create_table()?;
        for (name, code) in &plugin.mod_map {
            table.set(name.clone(), code.clone())?;
        }
        globals.set("QL_MODULE_TABLE", table)?;
        globals.set("QL_MODULE_TABLE_CACHE", plugin.lua.create_table()?)?;

        globals.set("QL_CLASSPATH_SEPARATOR", CLASSPATH_SEPARATOR.to_string())?;

        globals.set(
            "require",
            plugin.lua.create_function(|vm, name: String| {
                let globals = vm.globals();
                let table_cache: Table = globals.get("QL_MODULE_TABLE_CACHE")?;

                let table_cache_val: Value = table_cache.get(name.clone())?;
                if !table_cache_val.is_nil() {
                    return Ok(table_cache_val);
                }

                let table: Table = globals.get("QL_MODULE_TABLE")?;
                let table_code: String = table.get(name.clone())?;

                let load = vm.load(&table_code);
                let retval: Value = load.eval()?;

                Ok(retval)
            })?,
        )?;

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

    pub fn set_bytes(&self, bytes: &[u8], name: &str) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        let bytes = self.lua.create_string(bytes)?;
        globals.set(name, bytes)?;
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
        let func = self.lua.create_function(
            move |_,
                  (name, version, progress, args, current_dir): (
                String,
                i32,
                Option<LuaGenericProgress>,
                Vec<Value>,
                Option<String>,
            )| {
                let version = match version {
                    8 => JavaVersion::Java8,
                    16 => JavaVersion::Java16,
                    17 => JavaVersion::Java17,
                    21 => JavaVersion::Java21,
                    ver => {
                        return Err(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                            "Could not determine valid java version: {ver}. Valid inputs"
                        )))))
                    }
                };
                let arc = progress.map(|n| n.0.clone());
                let bin_path = RUNTIME
                    .block_on(get_java_binary(
                        version,
                        &name,
                        if let Some(sender) = &arc {
                            Some(sender)
                        } else {
                            None
                        },
                    ))
                    .map_err(|err| mlua::Error::ExternalError(Arc::new(err)))?;

                let args: Result<Vec<String>, mlua::Error> =
                    args.into_iter().map(|n| n.to_string()).collect();

                let mut command = std::process::Command::new(&bin_path);
                let command = if let Some(current_dir) = current_dir {
                    command.current_dir(&current_dir)
                } else {
                    &mut command
                };
                let command = match command.args(&args?).output() {
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

    fn fn_file_pick(&self, globals: &mlua::Table) -> Result<(), PluginError> {
        let func = self.lua.create_function(
            |lua, (name, filters, filter_name): (String, Vec<String>, String)| {
                let file = rfd::FileDialog::new()
                    .set_title(&name)
                    .add_filter(&filter_name, &filters)
                    .pick_file()
                    .ok_or(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                        "Could not pick file: {name}",
                    )))))?;

                let file_bytes = std::fs::read(&file)
                    .path(&file)
                    .map_err(|n| mlua::Error::ExternalError(Arc::new(n)))?;

                let lua_string = lua.create_string(&file_bytes)?;

                Ok(lua_string)
            },
        )?;

        globals.set("qlPickFile", func)?;

        Ok(())
    }

    fn resolve_deps(
        &mut self,
        json: &PluginJson,
        plugins_map: &HashMap<String, PluginJson>,
        plugins_top_dir: &Path,
        globals: &Table,
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
                file.load(&plugin_root_dir, &mut self.mod_map)?;
            }
            json.main_file.load(&plugin_root_dir, &mut self.mod_map)?;

            self.resolve_include_file(json, &plugin_root_dir, globals)?;

            self.resolve_deps(json, plugins_map, plugins_top_dir, globals)?;
        }
        Ok(())
    }

    fn resolve_include_file(
        &mut self,
        json: &PluginJson,
        plugin_root_dir: &Path,
        globals: &Table,
    ) -> Result<(), PluginError> {
        if let Some(includes) = &json.includes {
            for file in includes {
                let lua_file = plugin_root_dir.join(&file.filename);
                let lua_file = self
                    .lua
                    .create_string(std::fs::read(&lua_file).path(lua_file)?)?;
                globals.set(file.import.clone(), lua_file)?;
            }
        }
        Ok(())
    }

    fn fn_download(&self, whitelist: Vec<String>, globals: &Table) -> Result<(), PluginError> {
        let func = self
            .lua
            .create_function(move |vm, (url, user_agent): (String, bool)| {
                let is_whitelisted = whitelist.iter().any(|n| url.starts_with(n));
                if is_whitelisted {
                    let res = RUNTIME
                        .block_on(file_utils::download_file_to_bytes(&url, user_agent))
                        .map_err(|n| mlua::Error::ExternalError(Arc::new(n)))?;
                    Ok(vm.create_string(&res))
                } else {
                    Err(err_to_lua(format!(
                        "Url {url} is not in whitelist! This is a bug"
                    )))
                }
            })?;
        globals.set("qlDownload", func)?;
        Ok(())
    }
}

pub fn read_plugins() -> Result<(PathBuf, HashMap<String, PluginJson>), PluginError> {
    let plugins_top_dir = file_utils::get_launcher_dir_s()?.join("plugins");
    std::fs::create_dir_all(&plugins_top_dir).path(&plugins_top_dir)?;
    let plugins = std::fs::read_dir(&plugins_top_dir).path(&plugins_top_dir)?;

    let mut plugins_map = HashMap::new();

    for plugin in plugins {
        let plugin = plugin.map_err(|error| IoError::ReadDir {
            error,
            parent: plugins_top_dir.clone(),
        })?;
        let path = plugin.path();
        if path.is_file() {
            continue;
        }
        let json_path = path.join("index.json");
        let json = std::fs::read_to_string(&json_path).path(json_path)?;
        let json: PluginJson = serde_json::from_str(&json)?;
        let file_name = plugin.file_name();
        plugins_map.insert(file_name.to_str().unwrap().to_owned(), json);
    }

    Ok((plugins_top_dir, plugins_map))
}

fn create_lua() -> Result<Lua, PluginError> {
    let lua = Lua::new();
    lua.load_std_libs(StdLib::MATH | StdLib::PACKAGE | StdLib::STRING | StdLib::TABLE)?;

    let globals = lua.globals();
    // WTF: The dumb std lib loader thing doesn't prevent this bruh
    // Almost caused a CVE with this one.
    globals.set("io", mlua::Nil)?;
    globals.set("os", mlua::Nil)?;

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
    mlua::Error::ExternalError(Arc::new(StrErr(err.to_string())))
}

#[derive(Debug)]
pub struct StrErr(String);

impl std::fmt::Display for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StrErr {}
