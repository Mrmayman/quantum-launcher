//! # A crate for managing Minecraft servers
//!
//! **Not recommended to use this in your own projects!**
//!
//! This is a crate of
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher)
//! for managing Minecraft servers.

use std::path::PathBuf;

use ql_core::{impl_3_errs_jri, IoError, JsonError, RequestError};
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

const SERVER_ERR_PREFIX: &str = "while managing server:\n";

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("while downloading server\nserver version not found in manifest: {0}")]
    VersionNotFoundInManifest(String),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{SERVER_ERR_PREFIX}{0}")]
    JavaInstall(#[from] JavaInstallError),
    #[error(
        "{SERVER_ERR_PREFIX}couldn't find download field:\n(details.json).downloads.server is null"
    )]
    NoServerDownload,
    #[error("A server with that name already exists!")]
    ServerAlreadyExists,
    #[error("{SERVER_ERR_PREFIX}zip extract error:\n{0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("{SERVER_ERR_PREFIX}couldn't find forge shim file")]
    NoForgeShimFound,
    #[error("{SERVER_ERR_PREFIX}unsupported CPU architecture for ssh")]
    UnsupportedSSHArchitecture,
    #[error("{SERVER_ERR_PREFIX}couldn't convert PathBuf to str: {0:?}")]
    PathBufToStr(PathBuf),
}

impl_3_errs_jri!(ServerError, Json, Request, Io);
