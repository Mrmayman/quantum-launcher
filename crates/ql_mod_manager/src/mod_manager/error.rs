use ql_core::{IoError, JsonDownloadError, JsonFileError, RequestError};
use thiserror::Error;
use zip_extract::ZipError;

#[derive(Debug, Error)]
pub enum ModError {
    #[error(transparent)]
    RequestError(#[from] RequestError),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] IoError),
    #[error("no compatible version found for mod")]
    NoCompatibleVersionFound,
    #[error("no files found for mod")]
    NoFilesFound,
    #[error("couldn't add entry {1} to zip: {0}")]
    ZipIoError(std::io::Error, String),
    #[error("zip error: {0}")]
    Zip(#[from] ZipError),
    #[error("no minecraft entry found in curseforge API")]
    NoMinecraftInCurseForge,
}

impl From<JsonFileError> for ModError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}

impl From<JsonDownloadError> for ModError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::SerdeError(err) => err.into(),
            JsonDownloadError::RequestError(err) => err.into(),
        }
    }
}
