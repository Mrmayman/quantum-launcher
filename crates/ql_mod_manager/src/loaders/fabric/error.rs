use std::{path::PathBuf, sync::mpsc::SendError};

use ql_core::{GenericProgress, IoError, JsonFileError, RequestError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FabricInstallError {
    #[error("error installing fabric: {0}")]
    Io(#[from] IoError),
    #[error("error installing fabric: json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("error installing fabric: {0}")]
    RequestError(#[from] RequestError),
    #[error("error installing fabric: send error: {0}")]
    Send(#[from] SendError<GenericProgress>),
    #[error("error installing fabric: could not get path parent: {0:?}")]
    PathBufParentError(PathBuf),
    #[error("error installing fabric: zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("error installing fabric: zip write at {1}: {0}")]
    ZipEntryWriteError(std::io::Error, String),
    #[error("error installing fabric: zip read at {1}: {0}")]
    ZipEntryReadError(std::io::Error, String),
}

impl From<JsonFileError> for FabricInstallError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => Self::Json(err),
            JsonFileError::Io(err) => Self::Io(err),
        }
    }
}
