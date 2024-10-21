use std::{fmt::Display, num::ParseIntError, path::PathBuf, string::FromUtf8Error};

use ql_instances::{
    error::IoError, file_utils::RequestError, java_install::JavaInstallError,
    json_structs::JsonDownloadError,
};
use zip_extract::ZipExtractError;

use crate::instance_mod_installer::ChangeConfigError;

#[derive(Debug)]
pub enum ForgeInstallError {
    Io(IoError),
    Request(RequestError),
    Serde(serde_json::Error),
    NoForgeVersionFound,
    ParseIntError(ParseIntError),
    TempFile(std::io::Error),
    JavaInstallError(JavaInstallError),
    PathBufToStr(PathBuf),
    CompileError(String, String),
    InstallerError(String, String),
    Unpack200Error(String, String),
    FromUtf8Error(FromUtf8Error),
    LibraryParentError,
    ChangeConfigError(ChangeConfigError),
    NoInstallJson,
    ZipExtract(ZipExtractError),
}

impl Display for ForgeInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error installing forge: ")?;
        match self {
            ForgeInstallError::Io(err) => write!(f, "{err}"),
            ForgeInstallError::Request(err) => write!(f, "{err}"),
            ForgeInstallError::Serde(err) => write!(f, "{err}"),
            ForgeInstallError::NoForgeVersionFound => {
                write!(f, "no matching forge version found")
            }
            ForgeInstallError::ParseIntError(err) => write!(f, "{err}"),
            ForgeInstallError::TempFile(err) => {
                write!(f, "(tempfile): {err}")
            }
            ForgeInstallError::JavaInstallError(err) => {
                write!(f, "(java install): {err}")
            }
            ForgeInstallError::PathBufToStr(err) => {
                write!(f, "(pathbuf to str): {err:?}")
            }
            ForgeInstallError::CompileError(stdout, stderr) => {
                write!(f, "(compile error)\nSTDOUT = {stdout}\nSTDERR = {stderr}")
            }
            ForgeInstallError::InstallerError(stdout, stderr) => {
                write!(f, "(installer error)\nSTDOUT = {stdout}\nSTDERR = {stderr}")
            }
            ForgeInstallError::Unpack200Error(stdout, stderr) => {
                write!(f, "(unpack200 error)\nSTDOUT = {stdout}\nSTDERR = {stderr}")
            }
            ForgeInstallError::FromUtf8Error(err) => {
                write!(f, "(from utf8 error): {err}")
            }
            ForgeInstallError::LibraryParentError => {
                write!(f, "could not find parent directory of library")
            }
            ForgeInstallError::ChangeConfigError(err) => {
                write!(f, "(change config): {err}")
            }
            ForgeInstallError::NoInstallJson => {
                write!(f, "no install json found")
            }
            ForgeInstallError::ZipExtract(err) => {
                write!(f, "(zip extract): {err}")
            }
        }
    }
}

impl From<IoError> for ForgeInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<RequestError> for ForgeInstallError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<serde_json::Error> for ForgeInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<ParseIntError> for ForgeInstallError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl From<JavaInstallError> for ForgeInstallError {
    fn from(value: JavaInstallError) -> Self {
        Self::JavaInstallError(value)
    }
}

impl From<FromUtf8Error> for ForgeInstallError {
    fn from(value: FromUtf8Error) -> Self {
        Self::FromUtf8Error(value)
    }
}

impl From<ChangeConfigError> for ForgeInstallError {
    fn from(value: ChangeConfigError) -> Self {
        Self::ChangeConfigError(value)
    }
}

impl From<ZipExtractError> for ForgeInstallError {
    fn from(value: ZipExtractError) -> Self {
        Self::ZipExtract(value)
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

impl<T> Is404NotFound for Result<T, ForgeInstallError> {
    fn is_not_found(&self) -> bool {
        if let Err(ForgeInstallError::Request(RequestError::DownloadError { code, .. })) = &self {
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
