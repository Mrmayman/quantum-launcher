use std::path::PathBuf;

use ql_core::{impl_3_errs_jri, IoError, JsonError, RequestError};
use thiserror::Error;

const FABRIC_INSTALL_ERR_PREFIX: &str = "while installing Fabric:\n";

#[derive(Debug, Error)]
pub enum FabricInstallError {
    #[error("{FABRIC_INSTALL_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}{0}")]
    RequestError(#[from] RequestError),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}could not get parent of path: {0:?}")]
    PathBufParentError(PathBuf),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}zip error:\n{0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}zip write failed at {1}: {0}")]
    ZipEntryWriteError(std::io::Error, String),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}zip read failed at {1}: {0}")]
    ZipEntryReadError(std::io::Error, String),
    #[error("{FABRIC_INSTALL_ERR_PREFIX}no compatible version found for your instance")]
    NoVersionFound,
}

impl_3_errs_jri!(FabricInstallError, Json, RequestError, Io);
