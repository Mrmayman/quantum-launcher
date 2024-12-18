use std::{fmt::Display, path::PathBuf};

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

#[macro_export]
macro_rules! io_err {
    ($path:expr) => {
        |err: std::io::Error| $crate::IoError::Io {
            error: err,
            path: $path.to_owned(),
        }
    };
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
