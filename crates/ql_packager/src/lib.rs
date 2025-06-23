use std::{collections::HashSet, path::PathBuf};

use ql_core::{IoError, JsonError};
use ql_servers::ServerError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip_extract::ZipExtractError;

use ql_instances::DownloadError;

mod export;
mod import;
mod multimc;

pub use export::{EXCEPTIONS, export_instance};
pub use import::import_instance;

const PKG_ERR_PREFIX: &str = "while importing/exporting instance:\n";
#[derive(Debug, Error)]
pub enum InstancePackageError {
    #[error("{PKG_ERR_PREFIX}can't get filename of path {0:?}")]
    PathFileName(PathBuf),
    #[error("{PKG_ERR_PREFIX}path contains invalid unicode characters:\n{0:?}")]
    PathBufToStr(PathBuf),

    #[error("{PKG_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{PKG_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),

    #[error("{PKG_ERR_PREFIX}while creating base instance for import:\n{0}")]
    Download(#[from] DownloadError),
    #[error("{PKG_ERR_PREFIX}while creating new base server for import:\n{0}")]
    Server(#[from] ServerError),
    #[error("{PKG_ERR_PREFIX}while installing packaged loader:\n{0}")]
    Loader(String),

    #[error("{PKG_ERR_PREFIX}while extracting zip:\n{0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("{PKG_ERR_PREFIX}while dealing with zip:\n{0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("{PKG_ERR_PREFIX}while creating temporary directory:\n{0}")]
    TempDir(std::io::Error),
    #[error("{PKG_ERR_PREFIX}while adding to zip:\n{0}")]
    ZipIo(std::io::Error),
    #[error("{PKG_ERR_PREFIX}while parsing ini file:\n{0}")]
    Ini(#[from] ini::Error),
    #[error("{PKG_ERR_PREFIX}in ini file:\nentry {1:?} of section {0:?} is missing!")]
    IniFieldMissing(String, String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstanceInfo {
    pub instance_name: String,
    pub exceptions: HashSet<String>,
    pub is_server: bool,
}
