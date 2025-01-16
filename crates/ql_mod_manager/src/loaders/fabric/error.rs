use std::{fmt::Display, path::PathBuf, sync::mpsc::SendError};

use ql_core::{GenericProgress, IoError, JsonFileError, RequestError};

#[derive(Debug)]
pub enum FabricInstallError {
    Io(IoError),
    Json(serde_json::Error),
    RequestError(RequestError),
    Send(SendError<GenericProgress>),
    PathBufParentError(PathBuf),
    ZipError(zip::result::ZipError),
    ZipEntryWriteError(std::io::Error, String),
    ZipEntryReadError(std::io::Error, String),
}

impl From<IoError> for FabricInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for FabricInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<RequestError> for FabricInstallError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

impl From<SendError<GenericProgress>> for FabricInstallError {
    fn from(value: SendError<GenericProgress>) -> Self {
        Self::Send(value)
    }
}

impl From<JsonFileError> for FabricInstallError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => Self::Json(err),
            JsonFileError::Io(err) => Self::Io(err),
        }
    }
}

impl From<zip::result::ZipError> for FabricInstallError {
    fn from(value: zip::result::ZipError) -> Self {
        Self::ZipError(value)
    }
}

impl Display for FabricInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error installing fabric: ")?;
        match self {
            // Look, I'm not the best at programming.
            FabricInstallError::Io(err) => write!(f, "(io) {err}"),
            FabricInstallError::Json(err) => {
                write!(f, "(parsing json) {err}")
            }
            FabricInstallError::RequestError(err) => {
                write!(f, "(downloading file) {err}")
            }
            FabricInstallError::Send(err) => {
                write!(f, "(sending message) {err}")
            }
            FabricInstallError::PathBufParentError(path_buf) => {
                write!(f, "could not get parent of pathbuf: {path_buf:?}")
            }
            FabricInstallError::ZipError(err) => write!(f, "(zip file) {err}"),
            FabricInstallError::ZipEntryWriteError(err, path) => {
                write!(f, "error writing zip entry {path}: {err}")
            }
            FabricInstallError::ZipEntryReadError(err, path) => {
                write!(f, "error reading zip entry {path}: {err}")
            }
        }
    }
}
