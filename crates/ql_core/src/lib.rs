//! Core utilities shared between the various crates.
//!
//! # Contains
//! - Java auto-installer
//! - File and download utilities
//! - Error types
//! - JSON structs for version, instance config, Fabric, Forge, Optifine, etc
//! - Logging macros
//! - And much more

#![allow(clippy::cast_precision_loss)]

mod error;
/// Common utilities for working with files.
pub mod file_utils;
pub mod jarmod;
/// JSON structs for version, instance config, Fabric, Forge, Optifine, Quilt, Neoforge, etc.
pub mod json;
mod loader;
/// Logging macros.
pub mod print;
mod progress;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub use error::{
    DownloadError, IntoIoError, IntoJsonError, IntoStringError, IoError, JsonDownloadError,
    JsonError, JsonFileError,
};
pub use file_utils::{RequestError, LAUNCHER_DIR};
use futures::StreamExt;
use json::VersionDetails;
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

/// Perform multiple async tasks concurrently. Useful for things like
/// downloading lots of files at the same time.
///
/// # Calling
///
/// This takes in an `Iterator` of the `Future` of `async fn -> Result<T, E>`
/// and returns `Result<Vec<T>, E>`, where if any one of the
/// input functions failed the whole thing will fail.
///
/// Only if all the input functions succeed, it will return a `Vec`
/// of the output data.
///
/// # Example
/// ```no_run
/// # async fn download_file(url: &str) -> Result<String, String> {
/// #     Ok("Hello".to_owned())
/// # }
/// # async fn trying() -> Result<String, String> {
/// #   let files: [&str; 1] = ["test"];
/// do_jobs(files.iter().map(|url| {
///     // Async function that returns Result<T, E>
///     // No need to await
///     download_file(url)
/// })).await?;
/// # }
/// ```
///
/// # Errors
/// Returns whatever error the input function returns.
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

/// Retries a non-deterministic function
/// multiple (5) times if it fails.
///
/// Some functions are inherently non-deterministic
/// in nature, ie. doing the same thing multiple times
/// won't always produce the same result.
/// **For example**, network operations like *downloading
/// a file* are non-deterministic as they can randomly
/// fail for no reason, anytime.
///
/// So by repeating the function multiple times if it
/// fails, we reduce the failure rate, because
/// we could get lucky and succeed on the second try,
/// or the third try...
///
/// # Calling
/// This takes in an async closure that returns a `Result<T, E>`.
/// More specifically, it takes in an `Fn` closure, which can run
/// repeatedly but without storing any state.
///
/// # Example
/// ```no_run
/// # async fn download_file(url: &str) -> Result<String, String> {
/// #     Ok("Hi".to_owned())
/// # }
/// # async fn download_something_important() -> Result<String, String> {
/// retry(async || download_file("example.com/my_file").await).await
/// # }
/// ```
///
/// # Errors
/// Returns whatever error the original function returned.
pub async fn retry<T, E, Res, Func>(f: Func) -> Result<T, E>
where
    Res: Future<Output = Result<T, E>>,
    Func: Fn() -> Res,
{
    const LIMIT: usize = 5;
    let mut result = f().await;
    for _ in 0..LIMIT {
        if result.is_ok() {
            break;
        }
        result = f().await;
    }
    result
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
    pub fn get_instance_path(&self) -> PathBuf {
        match self {
            Self::Instance(name) => LAUNCHER_DIR.join("instances").join(name),
            Self::Server(name) => LAUNCHER_DIR.join("servers").join(name),
        }
    }

    #[must_use]
    pub fn get_dot_minecraft_path(&self) -> PathBuf {
        match self {
            InstanceSelection::Instance(name) => {
                LAUNCHER_DIR.join("instances").join(name).join(".minecraft")
            }
            InstanceSelection::Server(name) => LAUNCHER_DIR.join("servers").join(name),
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

#[derive(Debug, Clone, Copy)]
pub enum OptifineUniqueVersion {
    V1_5_2,
    V1_2_5,
    B1_7_3,
    B1_6_6,
}

impl OptifineUniqueVersion {
    #[must_use]
    pub fn get(instance: &InstanceSelection) -> Option<Self> {
        VersionDetails::load_s(&instance.get_instance_path()).and_then(|n| match n.id.as_str() {
            "1.5.2" => Some(OptifineUniqueVersion::V1_5_2),
            "1.2.5" => Some(OptifineUniqueVersion::V1_2_5),
            "b1.7.3" => Some(OptifineUniqueVersion::B1_7_3),
            "b1.6.6" => Some(OptifineUniqueVersion::B1_6_6),
            _ => None,
        })
    }

    #[must_use]
    pub fn get_url(&self) -> (&'static str, bool) {
        match self {
            OptifineUniqueVersion::V1_5_2 => ("https://optifine.net/adloadx?f=OptiFine_1.5.2_HD_U_D5.zip", false),
            OptifineUniqueVersion::V1_2_5 => ("https://optifine.net/adloadx?f=OptiFine_1.5.2_HD_U_D2.zip", false),
            OptifineUniqueVersion::B1_7_3 => ("https://b2.mcarchive.net/file/mcarchive/47df260a369eb2f79750ec24e4cfd9da93b9aac076f97a1332302974f19e6024/OptiFine_1_7_3_HD_G.zip", true),
            OptifineUniqueVersion::B1_6_6 => ("https://optifine.net/adloadx?f=beta_OptiFog_Optimine_1.6.6.zip", false),
        }
    }
}

pub fn get_jar_path(
    version_json: &VersionDetails,
    instance_dir: &Path,
    optifine_jar: Option<&Path>,
) -> PathBuf {
    optifine_jar.map_or_else(
        || {
            instance_dir
                .join(".minecraft/versions")
                .join(&version_json.id)
                .join(format!("{}.jar", version_json.id))
        },
        Path::to_owned,
    )
}
