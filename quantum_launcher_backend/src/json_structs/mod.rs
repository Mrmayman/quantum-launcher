use crate::file_utils::RequestError;

pub mod json_fabric;
pub mod json_instance_config;
pub mod json_java_list;
pub mod json_manifest;
pub mod json_profiles;
pub mod json_version;

pub enum JsonDownloadError {
    RequestError(RequestError),
    SerdeError(serde_json::Error),
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
