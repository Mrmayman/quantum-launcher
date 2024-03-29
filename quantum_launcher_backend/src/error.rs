use std::{fmt::Display, path::PathBuf, string::FromUtf8Error, sync::mpsc::SendError};

use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use std::io::Error as IoError;

use crate::{download::Progress, json_structs::json_version::VersionDetails};

#[derive(Debug)]
pub enum LauncherError {
    ConfigDirNotFound,
    IoError(std::io::Error),
    InstanceNotFound,
    UsernameIsInvalid(String),
    InstanceAlreadyExists,
    ReqwestError(reqwest::Error),
    ReqwestStatusError(reqwest::StatusCode, reqwest::Url),
    SerdeJsonError(serde_json::Error),
    SerdeFieldNotFound(&'static str),
    VersionNotFoundInManifest(String),
    JavaVersionIsEmptyError,
    JavaVersionConvertCmdOutputToStringError(FromUtf8Error),
    JavaVersionImproperVersionPlacement(String),
    JavaVersionParseToNumberError(String),
    VersionJsonNoArgumentsField(VersionDetails),
    PathBufToString(PathBuf),
    RequiredJavaVersionNotFound(usize),
    DownloadProgressMspcError(SendError<Progress>),
}

pub type LauncherResult<T> = Result<T, LauncherError>;

macro_rules! impl_error {
    ($from:ident, $to:ident) => {
        impl From<$from> for LauncherError {
            fn from(value: $from) -> Self {
                LauncherError::$to(value)
            }
        }
    };
}

impl_error!(ReqwestError, ReqwestError);
impl_error!(IoError, IoError);
impl_error!(SerdeJsonError, SerdeJsonError);
impl_error!(FromUtf8Error, JavaVersionConvertCmdOutputToStringError);

type ProgressSendError = SendError<Progress>;
impl_error!(ProgressSendError, DownloadProgressMspcError);

impl Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LauncherError::ConfigDirNotFound => match dirs::config_dir() {
                Some(n) => write!(f, "Config directory at {n:?} not accessible"),
                None => write!(
                    f,
                    "Config directory not found (AppData/Roaming on Windows, ~/.config/ on Linux)"
                ),
            },
            LauncherError::IoError(n) => write!(f, "IO Error: {n}"),
            LauncherError::InstanceNotFound => write!(f, "Selected Instance not found"),
            LauncherError::InstanceAlreadyExists => {
                write!(f, "Cannot create instance as it already exists")
            }
            LauncherError::ReqwestError(n) => write!(f, "Network error: {}", n),
            LauncherError::ReqwestStatusError(code, url) => write!(
                f,
                "Network status error when reading url {} : {code}",
                url.as_str(),
            ),
            LauncherError::SerdeJsonError(n) => write!(f, "JSON Error: {n}"),
            LauncherError::SerdeFieldNotFound(n) => write!(f, "JSON Field not found: {n}"),
            LauncherError::VersionNotFoundInManifest(n) => {
                write!(f, "Version {n} was not found in manifest JSON")
            }
            LauncherError::JavaVersionIsEmptyError => write!(
                f,
                "Got empty or invalid response when checking Java version"
            ),
            LauncherError::JavaVersionConvertCmdOutputToStringError(n) => {
                write!(f, "Java version message contains invalid characters: {n}")
            }
            LauncherError::JavaVersionImproperVersionPlacement(n) => write!(
                f,
                "Java version has invalid layout, could not read the version number: {n}"
            ),
            LauncherError::JavaVersionParseToNumberError(n) => {
                write!(f, "Could not convert Java version to a number: {n}")
            }
            LauncherError::VersionJsonNoArgumentsField(n) => {
                write!(f, "Version JSON does not have any arguments field:\n{n:?}")
            }
            LauncherError::PathBufToString(n) => write!(
                f,
                "Could not convert an OS path to String, may contain invalid characters: {n:?}"
            ),
            LauncherError::RequiredJavaVersionNotFound(ver) => write!(
                f,
                "The Java version ({ver}) required by the Minecraft version was not found"
            ),
            LauncherError::UsernameIsInvalid(n) => write!(f, "Username is invalid: {n}"),
            LauncherError::DownloadProgressMspcError(n) => {
                write!(f, "Could not send download progress: {n}")
            }
        }
    }
}
