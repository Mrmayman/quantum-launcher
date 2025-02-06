#[derive(Debug)]
pub enum PluginError {
    Mlua(mlua::Error),
}

impl From<mlua::Error> for PluginError {
    fn from(value: mlua::Error) -> Self {
        Self::Mlua(value)
    }
}
