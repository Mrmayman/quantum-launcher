#[derive(Debug)]
pub enum LauncherError {
    ConfigDirNotFound,
    IoError(std::io::Error),
    InstanceNotFound,
    InstanceAlreadyExists,
    ReqwestError(reqwest::Error),
    ReqwestStatusError(reqwest::StatusCode),
    SerdeJsonError(serde_json::Error),
    SerdeFieldNotFound(&'static str),
    VersionNotFoundInManifest(String),
}

pub type LauncherResult<T> = Result<T, LauncherError>;

impl From<reqwest::Error> for LauncherError {
    fn from(value: reqwest::Error) -> Self {
        LauncherError::ReqwestError(value)
    }
}

impl From<std::io::Error> for LauncherError {
    fn from(value: std::io::Error) -> Self {
        LauncherError::IoError(value)
    }
}

impl From<serde_json::Error> for LauncherError {
    fn from(value: serde_json::Error) -> Self {
        LauncherError::SerdeJsonError(value)
    }
}
