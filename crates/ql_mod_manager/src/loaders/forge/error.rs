use std::{num::ParseIntError, path::PathBuf, string::FromUtf8Error};

use ql_core::{impl_3_errs_jri, IoError, JsonError, RequestError};
use ql_java_handler::JavaInstallError;
use thiserror::Error;
use zip_extract::ZipExtractError;

const FORGE_INSTALL_ERR_PREFIX: &str = "while installing Forge:\n";

#[derive(Debug, Error)]
pub enum ForgeInstallError {
    #[error("{FORGE_INSTALL_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}no compatible forge version found!\n\nForge/NeoForge may be unsupported for this Minecraft version")]
    NoForgeVersionFound,
    #[error("{FORGE_INSTALL_ERR_PREFIX}error parsing int number:\n{0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}{0}")]
    JavaInstallError(#[from] JavaInstallError),
    #[error(
        "{FORGE_INSTALL_ERR_PREFIX}couldn't convert path to string (invalid characters):\n{0:?}"
    )]
    PathBufToStr(PathBuf),
    #[error("{FORGE_INSTALL_ERR_PREFIX}error compiling installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    CompileError(String, String),
    #[error("{FORGE_INSTALL_ERR_PREFIX}error running installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    InstallerError(String, String),
    #[error("{FORGE_INSTALL_ERR_PREFIX}couldn't convert bytes to string: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("{FORGE_INSTALL_ERR_PREFIX}couldn't find parent directory of library")]
    LibraryParentError,
    #[error("{FORGE_INSTALL_ERR_PREFIX}no install json found for Minecraft version: {0}\n\nThis is a bug! Please report!")]
    NoInstallJson(String),
    #[error("while installing neoforge:\nwhile checking if NeoForge supports the current version:\ncouldn't parse version release date:\n{0}")]
    ChronoTime(#[from] chrono::ParseError),
    #[error("neoforge only supports Minecraft 1.20.2 and above, your version is outdated")]
    NeoforgeOutdatedMinecraft,

    #[error("{FORGE_INSTALL_ERR_PREFIX}zip extract: {0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{FORGE_INSTALL_ERR_PREFIX}couldn't read file {1} from zip:\n{0}")]
    ZipIoError(std::io::Error, String),
}

impl_3_errs_jri!(ForgeInstallError, Json, Request, Io);

pub trait Is404NotFound {
    fn is_not_found(&self) -> bool;
}

impl<T, E: Is404NotFound> Is404NotFound for Result<T, E> {
    fn is_not_found(&self) -> bool {
        if let Err(err) = &self {
            err.is_not_found()
        } else {
            false
        }
    }
}

impl Is404NotFound for ForgeInstallError {
    fn is_not_found(&self) -> bool {
        if let ForgeInstallError::Request(RequestError::DownloadError { code, .. }) = &self {
            code.as_u16() == 404
        } else {
            false
        }
    }
}

impl Is404NotFound for RequestError {
    fn is_not_found(&self) -> bool {
        if let RequestError::DownloadError { code, .. } = &self {
            code.as_u16() == 404
        } else {
            false
        }
    }
}
