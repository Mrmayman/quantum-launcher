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
/// JSON structs for version, instance config, Fabric, Forge, OptiFine, etc.
pub mod json;
mod loader;
/// Logging macros.
pub mod print;
mod progress;

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub use error::{
    DownloadError, IntoIoError, IntoStringError, IoError, JsonDownloadError, JsonFileError,
};
pub use file_utils::RequestError;
use futures::StreamExt;
pub use loader::Loader;
pub use print::{logger_finish, LogType, LoggingState, LOGGER};
pub use progress::{DownloadProgress, GenericProgress, Progress};

pub const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

/// To prevent spawning of terminal (windows only).
///
/// Takes in a &mut Command (both `tokio` or `std` will do).
#[macro_export]
macro_rules! no_window {
    ($cmd:expr) => {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // 0x08000000 => CREATE_NO_WINDOW
            $cmd = $cmd.creation_flags(0x08000000);
        }
    };
}

pub static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Perform multiple async tasks concurrently.
pub async fn do_jobs<T, E>(
    results: impl Iterator<Item = impl std::future::Future<Output = Result<T, E>>>,
) -> Result<Vec<T>, E> {
    const JOBS: usize = 64;
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut outputs = Vec::new();

    for result in results {
        tasks.push(result);
        if tasks.len() > JOBS {
            if let Some(task) = tasks.next().await {
                outputs.push(task?);
            }
        }
    }

    while let Some(task) = tasks.next().await {
        outputs.push(task?);
    }
    Ok(outputs)
}

#[derive(Clone, Debug)]
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
            Self::Instance(ref mut n) | Self::Server(ref mut n) => name.clone_into(n),
        }
    }
}

pub const IS_ARM_LINUX: bool = cfg!(target_arch = "aarch64") && cfg!(target_os = "linux");
// pub const IS_ARM_LINUX: bool = true;

pub const LAUNCHER_VERSION_NAME: &str = "0.4.1";

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ModId {
    Modrinth(String),
    Curseforge(String),
}

impl ModId {
    #[must_use]
    pub fn get_internal_id(&self) -> &str {
        match self {
            ModId::Modrinth(n) | ModId::Curseforge(n) => n,
        }
    }

    #[must_use]
    pub fn get_index_str(&self) -> String {
        match self {
            ModId::Modrinth(n) => n.clone(),
            ModId::Curseforge(n) => format!("CF:{n}"),
        }
    }

    #[must_use]
    pub fn from_index_str(n: &str) -> Self {
        if n.starts_with("CF:") {
            ModId::Curseforge(n.strip_prefix("CF:").unwrap_or(n).to_owned())
        } else {
            ModId::Modrinth(n.to_owned())
        }
    }

    #[must_use]
    pub fn from_pair(n: &str, t: StoreBackendType) -> Self {
        match t {
            StoreBackendType::Modrinth => Self::Modrinth(n.to_owned()),
            StoreBackendType::Curseforge => Self::Curseforge(n.to_owned()),
        }
    }

    #[must_use]
    pub fn to_pair(self) -> (String, StoreBackendType) {
        let backend = match self {
            ModId::Modrinth(_) => StoreBackendType::Modrinth,
            ModId::Curseforge(_) => StoreBackendType::Curseforge,
        };

        (self.get_internal_id().to_owned(), backend)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackendType {
    Modrinth,
    Curseforge,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum SelectedMod {
    Downloaded { name: String, id: ModId },
    Local { file_name: String },
}

/// Opens the file explorer or browser
/// (depending on path/link) to the corresponding link.
///
/// If you input a url (starting with `https://` for example),
/// this will open the link with your default browser.
///
/// If you input a path (for example, `C:\Users\Mrmayman\Documents\`)
/// this will open it in the file explorer.
///
/// # Panics
/// Only supported on windows, macOS and linux,
/// other platforms will **panic**.
#[allow(clippy::zombie_processes)]
pub fn open_file_explorer(path: &str) {
    use std::process::Command;

    info!("Opening link: {path}");
    if let Err(err) = Command::new(if cfg!(target_os = "linux") {
        "xdg-open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        panic!("Opening file explorer not supported on this platform.")
    })
    .arg(path)
    .spawn()
    {
        err!("Could not open link: {err}");
    }
}

// #[macro_export]
// macro_rules! gen_w {
//     ($fn_name:ident, $doc:literal, $ret:ty, ($($arg_name:ident: $arg_type:ty),*), ($($arg_pass:expr),*)) => {
//         paste::paste! {
//             #[doc = "[`"]
//             #[doc = $doc]
//             #[doc = "`] `_w` function\n\nSee [`quantum_launcher`] / `main.rs` documentation for more info on what `_w` function is"]
//             #[allow(clippy::missing_errors_doc)]
//             pub async fn [<$fn_name _w>] (
//                 $($arg_name: $arg_type),*
//             ) -> $ret {
//                 $crate::IntoStringError::strerr($fn_name(
//                     $($arg_pass),*
//                 ).await)
//             }
//         }
//     };
// }
