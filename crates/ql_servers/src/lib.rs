use std::path::PathBuf;

use ql_core::{IoError, JsonDownloadError, JsonFileError, RequestError};
use ql_java_handler::JavaInstallError;

mod create;
mod list_versions;
mod read_log;
mod run;
mod server_properties;
// mod ssh;
pub use create::{create_server, delete_server};
pub use list_versions::list;
pub use read_log::read_logs;
pub use run::run;
pub use server_properties::ServerProperties;
// pub use ssh::run_tunnel;

use thiserror::Error;
use zip_extract::ZipExtractError;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("server error: {0}")]
    JsonDownload(#[from] JsonDownloadError),
    #[error("server error: {0}")]
    Request(#[from] RequestError),
    #[error("server version not found in manifest: {0}")]
    VersionNotFoundInManifest(String),
    #[error("server error: json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("server error: {0}")]
    Io(#[from] IoError),
    #[error("server error: {0}")]
    JavaInstall(#[from] JavaInstallError),
    #[error("couldn't find server download field\n(details.json).downloads.server is null")]
    NoServerDownload,
    #[error("server already exists")]
    ServerAlreadyExists,
    #[error("server error: zip extract: {0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("server error: couldn't find forge shim file")]
    NoForgeShimFound,
    #[error("server error: unsupported CPU architecture for ssh")]
    UnsupportedSSHArchitecture,
    #[error("server error: couldn't convert PathBuf to str: {0:?}")]
    PathBufToStr(PathBuf),
}

impl From<JsonFileError> for ServerError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(error) => Self::SerdeJson(error),
            JsonFileError::Io(error) => Self::Io(error),
        }
    }
}
