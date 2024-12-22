use std::fmt::Display;

use ql_core::{IoError, JsonDownloadError, RequestError};

mod create;
mod list_versions;
pub use create::{create_server, create_server_wrapped, ServerCreateProgress};
pub use list_versions::list_versions;

pub enum ServerError {
    JsonDownload(JsonDownloadError),
    Request(RequestError),
    VersionNotFoundInManifest(String),
    SerdeJson(serde_json::Error),
    Io(IoError),
    NoServerDownload,
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
        }
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
