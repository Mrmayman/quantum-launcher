use std::{fmt::Display, path::PathBuf, sync::mpsc::SendError};

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

#[derive(Debug)]
pub enum IoError {
    Io {
        error: std::io::Error,
        path: PathBuf,
    },
    ConfigDirNotFound,
}

impl Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io { error, path } => write!(f, "at path {path:?}, error {error}"),
            IoError::ConfigDirNotFound => write!(f, "config directory not found"),
        }
    }
}

pub trait IntoIoError<T> {
    fn path(self, p: impl Into<PathBuf>) -> Result<T, IoError>;
}

impl<T> IntoIoError<T> for std::io::Result<T> {
    fn path(self, p: impl Into<PathBuf>) -> Result<T, IoError> {
        self.map_err(|err: std::io::Error| IoError::Io {
            error: err,
            path: (p.into()).clone(),
        })
    }
}

#[derive(Debug)]
pub enum JsonDownloadError {
    RequestError(RequestError),
    SerdeError(serde_json::Error),
}

impl Display for JsonDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonDownloadError::RequestError(err) => {
                write!(f, "error downloading JSON: {err}")
            }
            JsonDownloadError::SerdeError(err) => {
                write!(f, "error downloading JSON: could not parse JSON: {err}")
            }
        }
    }
}

impl From<RequestError> for JsonDownloadError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

impl From<serde_json::Error> for JsonDownloadError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(value)
    }
}

#[derive(Debug)]
pub enum JsonFileError {
    SerdeError(serde_json::Error),
    Io(IoError),
}

impl Display for JsonFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonFileError::SerdeError(err) => write!(f, "error parsing json: {err}"),
            JsonFileError::Io(err) => write!(f, "error reading/writing json from file: {err}"),
        }
    }
}

impl From<serde_json::Error> for JsonFileError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(value)
    }
}

impl From<IoError> for JsonFileError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub enum DownloadError {
    Json(serde_json::Error),
    Request(RequestError),
    Io(IoError),
    InstanceAlreadyExists,
    SendProgress(SendError<DownloadProgress>),
    VersionNotFoundInManifest(String),
    SerdeFieldNotFound(String),
    NativesExtractError(zip_extract::ZipExtractError),
    NativesOutsideDirRemove,
    DownloadClassicZip,
}

impl From<serde_json::Error> for DownloadError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<RequestError> for DownloadError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<IoError> for DownloadError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<SendError<DownloadProgress>> for DownloadError {
    fn from(value: SendError<DownloadProgress>) -> Self {
        Self::SendProgress(value)
    }
}

impl From<JsonDownloadError> for DownloadError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => DownloadError::from(err),
            JsonDownloadError::SerdeError(err) => DownloadError::from(err),
        }
    }
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "download error: ")?;
        match self {
            DownloadError::Json(err) => write!(f, "json error {err}"),
            DownloadError::Request(err) => write!(f, "{err}"),
            DownloadError::Io(err) => write!(f, "{err}"),
            DownloadError::InstanceAlreadyExists => {
                write!(f, "instance already exists")
            }
            DownloadError::SendProgress(err) => write!(f, "send error: {err}"),
            DownloadError::VersionNotFoundInManifest(err) => {
                write!(f, "version not found in manifest {err}")
            }
            DownloadError::SerdeFieldNotFound(err) => write!(f, "serde field not found \"{err}\""),
            DownloadError::NativesExtractError(err) => {
                write!(f, "could not extract native libraries: {err}")
            }
            DownloadError::NativesOutsideDirRemove => write!(
                f,
                "tried to remove natives outside folder. POTENTIAL SECURITY RISK AVOIDED"
            ),
            DownloadError::DownloadClassicZip => {
                write!(f, "tried to download Minecraft classic server as a client!")
            }
        }
    }
}
