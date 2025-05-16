use std::path::PathBuf;

use thiserror::Error;

use crate::RequestError;

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
    #[allow(clippy::missing_errors_doc)]
    fn dir(self, p: impl Into<PathBuf>) -> Result<T, IoError>;
}

impl<T> IntoIoError<T> for std::io::Result<T> {
    fn path(self, p: impl Into<PathBuf>) -> Result<T, IoError> {
        self.map_err(|err: std::io::Error| IoError::Io {
            error: err.to_string(),
            path: (p.into()).clone(),
        })
    }

    fn dir(self, p: impl Into<PathBuf>) -> Result<T, IoError> {
        self.map_err(|err: std::io::Error| IoError::ReadDir {
            error: err.to_string(),
            parent: p.into(),
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
    #[error(transparent)]
    SerdeError(#[from] JsonError),
}

impl From<reqwest::Error> for JsonDownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(RequestError::ReqwestError(value))
    }
}

#[derive(Debug, Error)]
pub enum JsonFileError {
    #[error(transparent)]
    SerdeError(#[from] JsonError),
    #[error(transparent)]
    Io(#[from] IoError),
}

const JSON_ERR_PREFIX: &str = "could not parse JSON (this is a bug! please report):\n";

#[derive(Debug, Error)]
pub enum JsonError {
    #[error("{JSON_ERR_PREFIX}while parsing JSON:\n{error}\n\n{json}")]
    From {
        error: serde_json::Error,
        json: String,
    },
    #[error("{JSON_ERR_PREFIX}while converting object to JSON:\n{error}")]
    To { error: serde_json::Error },
}

pub trait IntoJsonError<T> {
    #[allow(clippy::missing_errors_doc)]
    fn json(self, p: String) -> Result<T, JsonError>;
    #[allow(clippy::missing_errors_doc)]
    fn json_to(self) -> Result<T, JsonError>;
}

impl<T> IntoJsonError<T> for Result<T, serde_json::Error> {
    fn json(self, json: String) -> Result<T, JsonError> {
        self.map_err(|error: serde_json::Error| JsonError::From { error, json })
    }

    fn json_to(self) -> Result<T, JsonError> {
        self.map_err(|error: serde_json::Error| JsonError::To { error })
    }
}
