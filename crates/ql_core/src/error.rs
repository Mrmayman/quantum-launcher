use std::{path::PathBuf, sync::mpsc::SendError};

use thiserror::Error;

use crate::{DownloadProgress, RequestError};

// macro_rules! impl_error {
//     ($from:ident, $to:ident) => {
//         impl From<$from> for LauncherError {
//             fn from(value: $from) -> Self {
//                 LauncherError::$to(value)
//             }
//         }
//     };
// }

// impl_error!(JsonDownloadError, JsonDownloadError);

#[derive(Clone, Debug, Error)]
pub enum IoError {
    #[error("at path {path:?}, error: {error}")]
    Io { error: String, path: PathBuf },
    #[error("couldn't read directory {parent:?}, error {error}")]
    ReadDir { error: String, parent: PathBuf },
    #[error("config or AppData directory not found")]
    ConfigDirNotFound,
    #[error("directory is outside parent directory. POTENTIAL SECURITY RISK AVOIDED")]
    DirEscapeAttack,
}

pub trait IntoIoError<T> {
    #[allow(clippy::missing_errors_doc)]
    fn path(self, p: impl Into<PathBuf>) -> Result<T, IoError>;
}

impl<T> IntoIoError<T> for std::io::Result<T> {
    fn path(self, p: impl Into<PathBuf>) -> Result<T, IoError> {
        self.map_err(|err: std::io::Error| IoError::Io {
            error: err.to_string(),
            path: (p.into()).clone(),
        })
    }
}

pub trait IntoStringError<T> {
    #[allow(clippy::missing_errors_doc)]
    fn strerr(self) -> Result<T, String>;
}

impl<T, E: ToString> IntoStringError<T> for Result<T, E> {
    fn strerr(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}

#[derive(Debug, Error)]
pub enum JsonDownloadError {
    #[error(transparent)]
    RequestError(#[from] RequestError),
    #[error("json error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

impl From<reqwest::Error> for JsonDownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(RequestError::ReqwestError(value))
    }
}

#[derive(Debug, Error)]
pub enum JsonFileError {
    #[error("json error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] IoError),
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("json error {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Request(#[from] RequestError),
    #[error(transparent)]
    Io(#[from] IoError),
    #[error("instance already exists!")]
    InstanceAlreadyExists,
    #[error("send error: {0}")]
    SendProgress(#[from] SendError<DownloadProgress>),
    #[error("version not found in manifest.json: {0}")]
    VersionNotFoundInManifest(String),
    #[error("json field not found \"{0}\"")]
    SerdeFieldNotFound(String),
    #[error("could not extract native libraries: {0}")]
    NativesExtractError(#[from] zip_extract::ZipExtractError),
    #[error("tried to remove natives outside folder. POTENTIAL SECURITY RISK AVOIDED")]
    NativesOutsideDirRemove,
    #[error("tried to download Minecraft classic server as a client!")]
    DownloadClassicZip,
}

impl From<JsonDownloadError> for DownloadError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => DownloadError::from(err),
            JsonDownloadError::SerdeError(err) => DownloadError::from(err),
        }
    }
}
