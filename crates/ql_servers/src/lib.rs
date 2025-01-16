use std::fmt::Display;

use ql_core::{IoError, JavaInstallError, JsonDownloadError, RequestError};

mod create;
mod list_versions;
mod read_log;
mod run;
mod server_properties;
// mod ssh;
pub use create::{create_server, create_server_w, delete_server};
pub use list_versions::list_versions;
pub use read_log::{read_logs, read_logs_w};
pub use run::{run, run_w};
pub use server_properties::ServerProperties;
// pub use ssh::run_tunnel;

use zip_extract::ZipExtractError;

pub enum ServerError {
    JsonDownload(JsonDownloadError),
    Request(RequestError),
    VersionNotFoundInManifest(String),
    SerdeJson(serde_json::Error),
    Io(IoError),
    JavaInstall(JavaInstallError),
    NoServerDownload,
    ServerAlreadyExists,
    ZipExtract(ZipExtractError),
    NoForgeShimFound,
    UnsupportedSSHArchitecture,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "server error: ")?;
        match self {
            ServerError::JsonDownload(err) => write!(f, "{err}"),
            ServerError::Request(err) => write!(f, "{err}"),
            ServerError::VersionNotFoundInManifest(version) => {
                write!(f, "could not find version {version} in manifest JSON")
            }
            ServerError::SerdeJson(err) => write!(f, "(json) {err}"),
            ServerError::Io(err) => write!(f, "(io) {err}"),
            ServerError::NoServerDownload => write!(
                f,
                "could not find server download field\n(details.json).downloads.server is null"
            ),
            ServerError::JavaInstall(err) => {
                write!(f, "{err}")
            }
            ServerError::ServerAlreadyExists => write!(f, "server already exists"),
            ServerError::ZipExtract(err) => write!(f, "zip extract: {err}"),
            ServerError::NoForgeShimFound => write!(f, "could not find forge shim file"),
            ServerError::UnsupportedSSHArchitecture => {
                write!(f, "unsupported CPU architecture (ssh)")
            }
        }
    }
}

impl From<ZipExtractError> for ServerError {
    fn from(e: ZipExtractError) -> Self {
        Self::ZipExtract(e)
    }
}

impl From<JsonDownloadError> for ServerError {
    fn from(e: JsonDownloadError) -> Self {
        Self::JsonDownload(e)
    }
}

impl From<RequestError> for ServerError {
    fn from(e: RequestError) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}

impl From<IoError> for ServerError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<JavaInstallError> for ServerError {
    fn from(e: JavaInstallError) -> Self {
        Self::JavaInstall(e)
    }
}
