use ql_core::IoError;

#[derive(Debug)]
pub enum PluginError {
    Mlua(mlua::Error),
    TokioRuntime(std::io::Error),
    Io(IoError),
    Serde(serde_json::Error),
    PluginNotFound(String, Option<String>),
}

impl From<mlua::Error> for PluginError {
    fn from(value: mlua::Error) -> Self {
        Self::Mlua(value)
    }
}

impl From<IoError> for PluginError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
