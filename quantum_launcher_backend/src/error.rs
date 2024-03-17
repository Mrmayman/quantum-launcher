use std::{path::PathBuf, string::FromUtf8Error};

use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use std::io::Error as IoError;

use crate::json_structs::json_version::VersionDetails;

#[derive(Debug)]
pub enum LauncherError {
    ConfigDirNotFound,
    IoError(std::io::Error),
    InstanceNotFound,
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
    RequiredJavaVersionNotFound,
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

// impl From<FromUtf8Error> for LauncherError {
//     fn from(value: FromUtf8Error) -> Self {
//         LauncherError::FromUtf8Error(value)
//     }
// }
