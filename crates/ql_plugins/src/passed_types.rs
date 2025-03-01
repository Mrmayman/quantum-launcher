use std::{
    fs::ReadDir,
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex},
};

use mlua::{FromLua, Lua, MetaMethod, UserData, UserDataMethods, Value};
use ql_core::{file_utils, GenericProgress, InstanceSelection, IntoIoError};

use crate::{plugin::err_to_lua, PluginError};

#[derive(Clone)]
pub struct LuaGenericProgress(pub Arc<Sender<GenericProgress>>);

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

#[derive(Clone)]
pub struct SelectedInstance {
    pub instance: InstanceSelection,
    pub path: PathBuf,
    pub dot_mc: bool,
}

impl SelectedInstance {
    pub fn get_path(&self) -> Result<PathBuf, mlua::Error> {
        let path = file_utils::get_launcher_dir_s().map_err(err_to_lua)?;
        let path = if self.dot_mc {
            self.instance.get_dot_minecraft_path(&path)
        } else {
            self.instance.get_instance_path(&path)
        };
        let new_path = path.join(&self.path);

        if new_path.starts_with(&path) {
            Ok(new_path)
        } else {
            Err(err_to_lua(format!(
                "Attempted to escape from fs sandbox: {new_path:?}"
            )))
        }
    }
}

impl FromLua for SelectedInstance {
    fn from_lua(value: Value, _: &Lua) -> Result<Self, mlua::Error> {
        match value {
            Value::UserData(ud) => {
                let instance = ud.borrow::<Self>()?;
                Ok((*instance).clone())
            }
            _ => unreachable!(),
        }
    }
}

impl UserData for SelectedInstance {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // methods.add_method("magnitude", |_, vec, ()| {
        //     let mag_squared = vec.0 * vec.0 + vec.1 * vec.1;
        //     Ok(mag_squared.sqrt())
        // });

        // methods.add_meta_function(MetaMethod::Add, |_, (vec1, vec2): (Vec2, Vec2)| {
        //     Ok(Vec2(vec1.0 + vec2.0, vec1.1 + vec2.1))
        // });

        methods.add_meta_function(MetaMethod::ToString, |_, instance: SelectedInstance| {
            let new_path = instance.get_path()?;
            let new_path = new_path.to_str().ok_or(err_to_lua(format!(
                "Could not convert path to string: {new_path:?}"
            )))?;
            Ok(new_path.to_owned())
        });

        methods.add_method("exists", |_, instance, ()| {
            let path = instance.get_path()?;
            Ok(path.exists())
        });

        methods.add_method("join", |_, instance, name: String| {
            let mut instance = instance.clone();
            instance.path = instance.path.join(name);
            Ok(instance)
        });

        methods.add_method("to_dot_mc_dir", |_, instance, ()| {
            let mut instance = instance.clone();
            instance.dot_mc = true;
            Ok(instance)
        });

        methods.add_method("to_instance_dir", |_, instance, ()| {
            let mut instance = instance.clone();
            instance.dot_mc = false;
            Ok(instance)
        });

        methods.add_method("write", |_, instance, bytes: mlua::String| {
            let path = instance.get_path()?;
            std::fs::write(&path, bytes.as_bytes())
                .path(&path)
                .map_err(err_to_lua)?;
            Ok(())
        });

        methods.add_method("create_dir", |_, instance, ()| {
            let path = instance.get_path()?;
            std::fs::create_dir_all(&path)
                .path(&path)
                .map_err(err_to_lua)?;
            Ok(())
        });

        methods.add_method("read", |vm, instance, ()| {
            let path = instance.get_path()?;
            let bytes = std::fs::read(&path).path(&path).map_err(err_to_lua)?;
            let string = vm.create_string(&bytes)?;
            Ok(string)
        });

        methods.add_method("is_dir", |_, instance, ()| {
            let path = instance.get_path()?;
            Ok(path.is_dir())
        });

        methods.add_method("is_file", |_, instance, ()| {
            let path = instance.get_path()?;
            Ok(path.is_file())
        });

        methods.add_method("exists", |_, instance, ()| {
            let path = instance.get_path()?;
            Ok(path.exists())
        });

        methods.add_method("filename", |vm, instance, ()| {
            let name = instance
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| vm.create_string(n).ok());

            Ok(name)
        });

        methods.add_method("iter_dir", |_, instance, ()| {
            let iterator = SelectedInstanceIterator::new(instance.clone()).map_err(err_to_lua)?;

            Ok(iterator)
        });

        methods.add_function("read_dir", |vm, instance: SelectedInstance| {
            let func: mlua::Function = vm
                .load(
                    r"
return function(path)
    local iter = path:iter_dir()
    return function()
        return iter:next()
    end
end",
                )
                .eval()?;

            let ret: mlua::Function = func.call(instance)?;

            Ok(ret)
        });
    }
}

#[derive(Clone)]
pub struct SelectedInstanceIterator {
    iter: Arc<Mutex<ReadDir>>,
    instance: SelectedInstance,
}

impl UserData for SelectedInstanceIterator {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("next", |_, instance: &mut SelectedInstanceIterator, ()| {
            let Some(item) = instance
                .iter
                .lock()
                .map_err(|n| err_to_lua(format!("Error locking dir reading mutex: {n}")))?
                .next()
            else {
                return Ok(None);
            };
            let item = item.map_err(|n| mlua::Error::ExternalError(Arc::new(n)))?;

            let instance_path = file_utils::get_launcher_dir_s().map_err(err_to_lua)?;
            let instance_path = if instance.instance.dot_mc {
                instance
                    .instance
                    .instance
                    .get_dot_minecraft_path(&instance_path)
            } else {
                instance.instance.instance.get_instance_path(&instance_path)
            };

            let path = item.path();
            let stripped_path = path
                .strip_prefix(instance_path)
                .map_err(|n| err_to_lua(format!("Error stripping dir reading path: {n}")))?;

            let new = SelectedInstance {
                instance: instance.instance.instance.clone(),
                path: stripped_path.to_owned(),
                dot_mc: instance.instance.dot_mc,
            };

            Ok(Some(new))
        });
    }
}

impl SelectedInstanceIterator {
    fn new(instance: SelectedInstance) -> Result<Self, PluginError> {
        let path = instance.get_path()?;
        let iter = std::fs::read_dir(&path).path(path)?;

        let this = Self {
            iter: Arc::new(Mutex::new(iter)),
            instance,
        };

        Ok(this)
    }
}
