use std::{num::ParseIntError, path::PathBuf, string::FromUtf8Error};

use ql_core::{IoError, JsonDownloadError, JsonFileError, RequestError};
use ql_java_handler::JavaInstallError;
use thiserror::Error;
use zip_extract::ZipExtractError;

#[derive(Debug, Error)]
pub enum ForgeInstallError {
    #[error("error installing forge: {0}")]
    Io(#[from] IoError),
    #[error("error installing forge: {0}")]
    Request(#[from] RequestError),
    #[error("error installing forge: json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("no matching forge version found")]
    NoForgeVersionFound,
    #[error("error installing forge: parse int error: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("error installing forge: tempfile: {0}")]
    TempFile(std::io::Error),
    #[error("error installing forge: {0}")]
    JavaInstallError(#[from] JavaInstallError),
    #[error("error installing forge: could not convert path to string: {0:?}")]
    PathBufToStr(PathBuf),
    #[error("error compiling forge installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    CompileError(String, String),
    #[error("error running forge installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    InstallerError(String, String),
    #[error("error installing forge: could not convert bytes to string: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("error installing forge: could not find parent directory of library")]
    LibraryParentError,
    #[error("error installing forge: no install json found")]
    NoInstallJson,
    #[error("error installing forge: zip extract: {0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("while installing neoforge: while checking if NeoForge supports the current version: could not parse version release date: {0}")]
    ChronoTime(#[from] chrono::ParseError),
    #[error("neoforge only supports Minecraft 1.20.2 and above, your version is outdated")]
    OutdatedMinecraft,
}

impl From<JsonFileError> for ForgeInstallError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::Io(err) => Self::Io(err),
            JsonFileError::SerdeError(err) => Self::Serde(err),
        }
    }
}

impl From<JsonDownloadError> for ForgeInstallError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => Self::Request(err),
            JsonDownloadError::SerdeError(err) => Self::Serde(err),
        }
    }
}

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
