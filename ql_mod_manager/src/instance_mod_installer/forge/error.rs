use std::{fmt::Display, num::ParseIntError, path::PathBuf, string::FromUtf8Error};

use ql_instances::{
    error::IoError, file_utils::RequestError, java_install::JavaInstallError,
    json_structs::JsonDownloadError,
};

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
}

impl Display for ForgeInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForgeInstallError::Io(err) => write!(f, "error installing forge: {err}"),
            ForgeInstallError::Request(err) => write!(f, "error installing forge: {err}"),
            ForgeInstallError::Serde(err) => write!(f, "error installing forge: {err}"),
            ForgeInstallError::NoForgeVersionFound => {
                write!(f, "error installing forge: no matching forge version found")
            }
            ForgeInstallError::ParseIntError(err) => write!(f, "error installing forge: {err}"),
            ForgeInstallError::TempFile(err) => {
                write!(f, "error installing forge (tempfile): {err}")
            }
            ForgeInstallError::JavaInstallError(err) => {
                write!(f, "error installing forge (java install): {err}")
            }
            ForgeInstallError::PathBufToStr(err) => {
                write!(f, "error installing forge (pathbuf to str): {err:?}")
            }
            ForgeInstallError::CompileError(stdout, stderr) => {
                write!(f, "error installing forge (compile error): STDOUT = {stdout}), STDERR = ({stderr})")
            }
            ForgeInstallError::InstallerError(stdout, stderr) => write!(
                f,
                "error installing forge (compile error): STDOUT = {stdout}), STDERR = ({stderr})"
            ),
            ForgeInstallError::Unpack200Error(stdout, stderr) => write!(
                f,
                "error installing forge (compile error): STDOUT = {stdout}), STDERR = ({stderr})"
            ),
            ForgeInstallError::FromUtf8Error(err) => {
                write!(f, "error installing forge (from utf8 error): {err}")
            }
            ForgeInstallError::LibraryParentError => write!(
                f,
                "error installing forge: could not find parent directory of library"
            ),
            ForgeInstallError::ChangeConfigError(err) => {
                write!(f, "error installing forge (change config): {err}")
            }
            ForgeInstallError::NoInstallJson => {
                write!(f, "error installing forge: no install json found")
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

impl From<JsonDownloadError> for ForgeInstallError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => Self::Request(err),
            JsonDownloadError::SerdeError(err) => Self::Serde(err),
        }
    }
}
