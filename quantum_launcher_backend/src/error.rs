use std::{fmt::Display, path::PathBuf, string::FromUtf8Error, sync::mpsc::SendError};

use serde_json::Error as SerdeJsonError;
use zip_extract::ZipExtractError;

use crate::{
    download::progress::DownloadProgress,
    file_utils::RequestError,
    json_structs::{json_version::VersionDetails, JsonDownloadError, JsonFileError},
};

#[derive(Debug)]
pub enum LauncherError {
    ConfigDirNotFound,
    InstanceNotFound,
    UsernameIsInvalid(String),
    InstanceAlreadyExists,
    RequestError(RequestError),
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
    DownloadProgressMspcError(SendError<DownloadProgress>),
    IoError(IoError),
    PathParentError(PathBuf),
    CommandError(std::io::Error),
    LatestFabricVersionNotFound,
    TempFileError(std::io::Error),
    NativesExtractError(ZipExtractError),
    NativesOutsideDirRemove,
    JsonDownloadError(JsonDownloadError),
    JsonFileError(JsonFileError),
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

impl_error!(JsonDownloadError, JsonDownloadError);
impl_error!(SerdeJsonError, SerdeJsonError);
impl_error!(FromUtf8Error, JavaVersionConvertCmdOutputToStringError);
impl_error!(JsonFileError, JsonFileError);
impl_error!(IoError, IoError);

type ProgressSendError = SendError<DownloadProgress>;
impl_error!(ProgressSendError, DownloadProgressMspcError);

impl Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LauncherError::ConfigDirNotFound => match dirs::config_dir() {
                Some(n) => write!(f, "Config directory at {n:?} not accessible"),
                None => write!(
                    f,
                    "config directory not found (AppData/Roaming on Windows, ~/.config/ on Linux)"
                ),
            },
            LauncherError::InstanceNotFound => write!(f, "Selected Instance not found"),
            LauncherError::InstanceAlreadyExists => {
                write!(f, "cannot create instance as it already exists")
            }
            LauncherError::SerdeJsonError(n) => write!(f, "JSON Error: {n}"),
            LauncherError::SerdeFieldNotFound(n) => write!(f, "JSON Field not found: {n}"),
            LauncherError::VersionNotFoundInManifest(n) => {
                write!(f, "version {n} was not found in manifest JSON")
            }
            LauncherError::JavaVersionIsEmptyError => write!(
                f,
                "got empty or invalid response when checking Java version"
            ),
            LauncherError::JavaVersionConvertCmdOutputToStringError(n) => {
                write!(f, "java version message contains invalid characters: {n}")
            }
            LauncherError::JavaVersionImproperVersionPlacement(n) => write!(
                f,
                "java version has invalid layout, could not read the version number: {n}"
            ),
            LauncherError::JavaVersionParseToNumberError(n) => {
                write!(f, "could not convert Java version to a number: {n}")
            }
            LauncherError::VersionJsonNoArgumentsField(n) => {
                write!(f, "version JSON does not have any arguments field:\n{n:?}")
            }
            LauncherError::PathBufToString(n) => write!(
                f,
                "could not convert an OS path to String, may contain invalid characters: {n:?}"
            ),
            LauncherError::RequiredJavaVersionNotFound(ver) => write!(
                f,
                "the Java version ({ver}) required by the Minecraft version was not found"
            ),
            LauncherError::UsernameIsInvalid(n) => write!(f, "username is invalid: {n}"),
            LauncherError::DownloadProgressMspcError(n) => {
                write!(f, "could not send download progress: {n}")
            }
            LauncherError::IoError(err) => write!(f, "{err}"),
            LauncherError::CommandError(n) => {
                write!(f, "IO error while trying to run Java command: {n}")
            }
            LauncherError::LatestFabricVersionNotFound => {
                write!(f, "could not find the latest Fabric loader version")
            }
            LauncherError::PathParentError(p) => write!(f, "could not get parent of path {p:?}"),
            LauncherError::TempFileError(err) => {
                write!(f, "could not create temporary file: {err}")
            }
            LauncherError::NativesExtractError(err) => {
                write!(f, "could not extract natives jar file as zip: {err}")
            }
            LauncherError::NativesOutsideDirRemove => write!(f, "tried to delete natives file outside QuantumLauncher/instances/INSTANCE/libraries/natives. POTENTIAL ATTACK AVOIDED"),
            LauncherError::RequestError(err) => write!(f, "{err}"),
            LauncherError::JsonDownloadError(err) => write!(f, "{err}"),
            LauncherError::JsonFileError(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Debug)]
pub enum IoError {
    Io {
        error: std::io::Error,
        path: PathBuf,
    },
    ConfigDirNotFound,
}

impl Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io { error, path } => write!(f, "at path {path:?}, error {error}"),
            IoError::ConfigDirNotFound => write!(f, "config directory not found"),
        }
    }
}

#[macro_export]
macro_rules! io_err {
    ($path:expr) => {
        |err: std::io::Error| $crate::error::IoError::Io {
            error: err,
            path: $path.to_owned(),
        }
    };
}
