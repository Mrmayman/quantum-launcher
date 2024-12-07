use std::{fmt::Display, path::PathBuf};

// macro_rules! impl_error {
//     ($from:ident, $to:ident) => {
//         impl From<$from> for LauncherError {
//             fn from(value: $from) -> Self {
//                 LauncherError::$to(value)
//             }
//         }
//     };
// }

// impl_error!(JsonDownloadError, JsonDownloadError);

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
