use std::fmt::Display;

use crate::{error::IoError, file_utils::RequestError};

pub mod json_fabric;
pub mod json_forge;
pub mod json_instance_config;
pub mod json_java_files;
pub mod json_java_list;
pub mod json_manifest;
pub mod json_optifine;
pub mod json_profiles;
pub mod json_version;

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
            JsonFileError::SerdeError(err) => write!(f, "error reading json from file: {err}"),
            JsonFileError::Io(err) => write!(f, "error reading json from file: {err}"),
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
