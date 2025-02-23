//! Core utilities shared between the various crates.
//!
//! # Contains
//! - Java auto-installer
//! - File and download utilities
//! - Error types
//! - JSON structs for version, instance config, Fabric, Forge, OptiFine, etc
//! - Logging macros
//! - And much more

mod error;
/// Common utilities for working with files.
pub mod file_utils;
mod java_install;
/// JSON structs for version, instance config, Fabric, Forge, OptiFine, etc.
pub mod json;
/// Logging macros.
pub mod print;
mod progress;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub use error::{DownloadError, IntoIoError, IoError, JsonDownloadError, JsonFileError};
pub use file_utils::{RequestError, MOCK_DIR_FAILURE};
use futures::StreamExt;
pub use java_install::{get_java_binary, JavaInstallError};
pub use progress::{DownloadProgress, GenericProgress, Progress};

pub const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

lazy_static::lazy_static! {
    static ref CLIENT: Arc<reqwest::Client> = Arc::new(reqwest::Client::new());
}

/// Limit on how many files to download concurrently.
const JOBS: usize = 64;

/// Perform multiple async tasks concurrently.
pub async fn do_jobs<ResultType>(
    results: impl Iterator<Item = impl std::future::Future<Output = ResultType>>,
) -> Vec<ResultType> {
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut outputs = Vec::new();

    for result in results {
        tasks.push(result);
        if tasks.len() > JOBS {
            if let Some(task) = tasks.next().await {
                outputs.push(task);
            }
        }
    }

    while let Some(task) = tasks.next().await {
        outputs.push(task);
    }
    outputs
}

#[derive(Clone)]
pub enum InstanceSelection {
    Instance(String),
    Server(String),
}

impl InstanceSelection {
    #[must_use]
    pub fn new(name: &str, is_server: bool) -> Self {
        if is_server {
            Self::Server(name.to_owned())
        } else {
            Self::Instance(name.to_owned())
        }
    }

    #[must_use]
    pub fn get_instance_path(&self, parent: &Path) -> PathBuf {
        match self {
            Self::Instance(name) => parent.join("instances").join(name),
            Self::Server(name) => parent.join("servers").join(name),
        }
    }

    #[must_use]
    pub fn get_dot_minecraft_path(&self, parent: &Path) -> PathBuf {
        match self {
            InstanceSelection::Instance(name) => {
                parent.join("instances").join(name).join(".minecraft")
            }
            InstanceSelection::Server(name) => parent.join("servers").join(name),
        }
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        match self {
            Self::Instance(name) | Self::Server(name) => name,
        }
    }

    #[must_use]
    pub fn is_server(&self) -> bool {
        matches!(self, Self::Server(_))
    }

    pub fn set_name(&mut self, name: &str) {
        match self {
            Self::Instance(ref mut n) | Self::Server(ref mut n) => *n = name.to_owned(),
        }
    }
}

pub const IS_ARM_LINUX: bool = cfg!(target_arch = "aarch64") && cfg!(target_os = "linux");
// pub const IS_ARM_LINUX: bool = true;

pub const LAUNCHER_VERSION_NAME: &str = "0.4.0";

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum SelectedMod {
    Downloaded { name: String, id: String },
    Local { file_name: String },
}

#[allow(clippy::zombie_processes)]
pub fn open_file_explorer(path: &str) {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    {
        _ = Command::new("xdg-open").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "windows")]
    {
        _ = Command::new("explorer").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "macos")]
    {
        _ = Command::new("open").arg(path).spawn().unwrap();
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    panic!("Opening file explorer not supported on this platform.")
}
