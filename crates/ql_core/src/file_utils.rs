use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::Metadata,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use futures::StreamExt;
use ql_reqwest::header::InvalidHeaderValue;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::fs::DirEntry;
use tokio_util::io::StreamReader;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

use crate::{
    error::{DownloadFileError, IoError},
    info_no_log, retry, IntoIoError, IntoJsonError, JsonDownloadError, CLIENT,
};

/// The path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qlportable.txt` is found, otherwise it uses the system config dir:
/// - `~/.config` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// Use [`get_launcher_dir`] for a non-panicking solution.
///
/// # Panics
/// - if config dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub static LAUNCHER_DIR: LazyLock<PathBuf> = LazyLock::new(|| get_launcher_dir().unwrap());

/// Returns the path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qlportable.txt` is found, otherwise it uses the system config dir:
/// - `~/.config` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// # Errors
/// - if config dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub fn get_launcher_dir() -> Result<PathBuf, IoError> {
    let launcher_directory = if let Some(n) = check_qlportable_file() {
        n.path
    } else {
        dirs::config_dir()
            .ok_or(IoError::ConfigDirNotFound)?
            .join("QuantumLauncher")
    };

    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;
    Ok(launcher_directory)
}

struct QlDirInfo {
    path: PathBuf,
}

fn line_and_body(input: &str) -> (String, String) {
    let mut lines = input.trim().lines();

    // Get the first line (if any)
    if let Some(first) = lines.next() {
        let rest = lines.collect::<Vec<_>>().join("\n");
        return (first.trim().to_owned(), rest);
    }

    (String::default(), String::default())
}

fn check_qlportable_file() -> Option<QlDirInfo> {
    const PORTABLE_FILENAME: &str = "qldir.txt";

    let places = [
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_owned)),
        std::env::current_dir().ok(),
        dirs::config_dir().map(|d| d.join("QuantumLauncher")),
    ];

    for (i, place) in places
        .into_iter()
        .enumerate()
        .filter_map(|(i, n)| n.map(|n| (i, n)))
    {
        let qldir_path = place.join(PORTABLE_FILENAME);
        let Ok(contents) = std::fs::read_to_string(&qldir_path) else {
            continue;
        };
        let (path, qldir_options) = line_and_body(&contents);

        let flags: HashSet<_> = qldir_options
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();

        if flags.contains("i_vulkan") {
            std::env::set_var("WGPU_BACKEND", "vulkan");
        } else if flags.contains("i_opengl") {
            std::env::set_var("WGPU_BACKEND", "opengl");
        } else if flags.contains("i_directx") {
            std::env::set_var("WGPU_BACKEND", "dx12");
        } else if flags.contains("i_metal") {
            std::env::set_var("WGPU_BACKEND", "metal");
        }

        let mut join_dir = !flags.contains("top");

        let path = if let (Some(stripped), Some(home)) = (path.strip_prefix("~"), dirs::home_dir())
        {
            home.join(stripped)
        } else if path == "." {
            join_dir = false;
            place
        } else if path.is_empty() && i < 2 {
            place
        } else {
            PathBuf::from(&path)
        };

        return Some(if join_dir {
            QlDirInfo {
                path: path.join("QuantumLauncher"),
            }
        } else {
            QlDirInfo { path }
        });
    }

    None
}

/// Returns whether the user is new to QuantumLauncher,
/// ie. whether they have never used the launcher before.
///
/// It checks whether the launcher directory does not exist.
#[must_use]
#[allow(clippy::doc_markdown)]
pub fn is_new_user() -> bool {
    let Some(config_directory) = dirs::config_dir() else {
        return false;
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    !launcher_directory.exists()
}

/// Downloads a file from the given URL into a `String`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_string(url: &str, user_agent: bool) -> Result<String, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<String, RequestError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header(
                "User-Agent",
                "Mrmayman/quantumlauncher (mrmayman.github.io/quantumlauncher)",
            );
        }
        let response = get.send().await?;
        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(url, user_agent).await).await
}

/// Downloads a file from the given URL into a JSON.
///
/// More specifically, it tries to parse the contents
/// into anything implementing `serde::Deserialize`
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_json<T: DeserializeOwned>(
    url: &str,
    user_agent: bool,
) -> Result<T, JsonDownloadError> {
    async fn inner<T: DeserializeOwned>(
        url: &str,
        user_agent: bool,
    ) -> Result<T, JsonDownloadError> {
        let text = download_file_to_string(url, user_agent).await?;
        Ok(serde_json::from_str(&text).json(text)?)
    }

    retry(async || inner(url, user_agent).await).await
}

/// Downloads a file from the given URL into a `Vec<u8>`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;
        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(url, user_agent).await).await
}

/// Downloads a file from the given URL and saves it to a path.
///
/// This uses `tokio` streams internally allowing for highly
/// efficient downloading.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
/// - `path`: the `&Path` to save the files to
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_path(
    url: &str,
    user_agent: bool,
    path: &Path,
) -> Result<(), DownloadFileError> {
    async fn inner(url: &str, user_agent: bool, path: &Path) -> Result<(), DownloadFileError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;

        if response.status().is_success() {
            let stream = response
                .bytes_stream()
                .map(|n| n.map_err(std::io::Error::other));
            let mut stream = StreamReader::new(stream);

            if let Some(parent) = path.parent() {
                if !parent.is_dir() {
                    tokio::fs::create_dir_all(&parent).await.path(parent)?;
                }
            }

            let mut file = tokio::fs::File::create(&path).await.path(path)?;
            tokio::io::copy(&mut stream, &mut file).await.path(path)?;
            Ok(())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            }
            .into())
        }
    }

    retry(async || inner(url, user_agent, path).await).await
}

/// Downloads a file from the given URL into a `Vec<u8>`,
/// with a custom user agent.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes_with_agent(
    url: &str,
    user_agent: &str,
) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: &str) -> Result<Vec<u8>, RequestError> {
        let response = CLIENT
            .get(url)
            .header("User-Agent", user_agent)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(url, user_agent).await).await
}

const NETWORK_ERROR_MSG: &str = r"
- Check your internet connection
- Check if you are behind a firewall/proxy
- Try doing the action again

";

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Download Error (code {code}){NETWORK_ERROR_MSG}Url: {url}")]
    DownloadError {
        code: ql_reqwest::StatusCode,
        url: ql_reqwest::Url,
    },
    #[error("Network Request Error{NETWORK_ERROR_MSG}{0}")]
    ReqwestError(#[from] ql_reqwest::Error),
    #[error("Download Error (invalid header value){NETWORK_ERROR_MSG}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

/// Sets the executable bit on a file.
///
/// This makes a file executable on Unix systems,
/// ie. it can be run as a program.
///
/// # Errors
/// Returns an error if:
/// - the file does not exist
/// - the user doesn't have permission to read the file metadata
/// - the user doesn't have permission to change the file metadata
#[cfg(target_family = "unix")]
pub async fn set_executable(path: &std::path::Path) -> Result<(), IoError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await.path(path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    tokio::fs::set_permissions(path, perms).await.path(path)
}

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

/// Creates a symbolic link (ie. the thing at `src` "points" to `dest`,
/// accessing `src` will actually access `dest`)
///
/// # Errors
/// (depending on platform):
/// - If `src` already exists
/// - If `dest` doesn't exist
/// - If user doesn't have permission for `src`
/// - If the path is invalid (part of path is not a directory for example)
/// - Other niche stuff (Read only filesystem, Running out of disk space)
pub fn create_symlink(src: &Path, dest: &Path) -> Result<(), IoError> {
    #[cfg(unix)]
    {
        symlink(src, dest).path(src)
    }

    #[cfg(windows)]
    {
        if src.is_dir() {
            symlink_dir(src, dest).path(src)
        } else {
            symlink_file(src, dest).path(src)
        }
    }
}

pub async fn clean_log_spam() -> Result<(), IoError> {
    const SIZE_LIMIT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

    let logs_dir = LAUNCHER_DIR.join("logs");
    if !logs_dir.is_dir() {
        tokio::fs::create_dir_all(&logs_dir).await.path(logs_dir)?;
        return Ok(());
    }
    let mut total_size = 0;
    let mut files: Vec<(DirEntry, Metadata)> = Vec::new();

    let mut read_dir = tokio::fs::read_dir(&logs_dir).await.dir(&logs_dir)?;

    while let Some(entry) = read_dir.next_entry().await.dir(&logs_dir)? {
        let metadata = entry.metadata().await.path(entry.path())?;
        if metadata.is_file() {
            total_size += metadata.len();
            files.push((entry, metadata));
        }
    }

    if total_size <= SIZE_LIMIT_BYTES {
        return Ok(());
    }

    info_no_log!(
        "Log exceeded {} MB, cleaning up",
        SIZE_LIMIT_BYTES / (1024 * 1024)
    );
    files.sort_unstable_by_key(|(_, metadata)| {
        metadata.modified().unwrap_or(std::time::SystemTime::now())
    });

    for (file, metadata) in files {
        let path = file.path();
        tokio::fs::remove_file(&path).await.path(path)?;
        total_size -= metadata.len();

        if total_size <= SIZE_LIMIT_BYTES {
            break;
        }
    }

    Ok(())
}

/// Recursively copies the contents of
/// the `src` dir to the `dst` dir.
///
/// File structure:
/// ```txt
/// src/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
/// To
/// ```txt
/// dst/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
///
/// # Errors
/// - `src` doesn't exist
/// - `dst` already has a dir with the same name as a file
/// - User doesn't have permissions for `src`/`dst` access
pub async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), IoError> {
    copy_dir_recursive_ext(src, dst, &[]).await?;
    Ok(())
}

/// Recursively copies the contents of
/// the `src` dir to the `dst` dir.
///
/// File structure:
/// ```txt
/// src/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
/// To
/// ```txt
/// dst/
///     a.txt
///     b.txt
///     c/
///         d.txt
/// ```
///
/// This function has a few extra
/// features compared to the non-ext one:
///
/// - Allows specifying exceptions for not copying.
/// - More coming in future.
///
/// # Errors
/// - `src` doesn't exist
/// - `dst` already has a dir with the same name as a file
/// - User doesn't have permissions for `src`/`dst` access
pub async fn copy_dir_recursive_ext(
    src: &Path,
    dst: &Path,
    exceptions: &[PathBuf],
) -> Result<(), IoError> {
    if src.is_file() {
        tokio::fs::copy(src, dst).await.path(src)?;
        return Ok(());
    }

    if !dst.exists() {
        tokio::fs::create_dir_all(dst).await.path(dst)?;
    }

    let mut dir = tokio::fs::read_dir(src).await.path(src)?;

    // Iterate over the directory entries
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if exceptions.contains(&path) {
            continue;
        }

        if path.is_dir() {
            // Recursively copy the subdirectory
            Box::pin(copy_dir_recursive(&path, &dest_path)).await?;
        } else {
            // Copy the file to the destination directory
            tokio::fs::copy(&path, &dest_path).await.path(path)?;
        }
    }

    Ok(())
}

/// Reads all the entries from a directory into a `Vec<String>`.
/// This includes both files and folders.
///
/// # Errors
/// - `dir` doesn't exist
/// - User doesn't have access to `dir`
///
/// Additionally, this skips any file/folder names
/// that has broken encoding (not UTF-8 or ASCII).
pub async fn read_filenames_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<String>, IoError> {
    let dir: &Path = dir.as_ref();
    let mut entries = tokio::fs::read_dir(dir).await.dir(dir)?;
    let mut filenames = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(|n| IoError::ReadDir {
        error: n.to_string(),
        parent: dir.to_owned(),
    })? {
        if let Some(name) = entry.file_name().to_str() {
            filenames.push(name.to_string());
        }
    }

    Ok(filenames)
}

/// Reads all the entries from a directory into a `Vec<String>`.
/// This includes both files and folders.
///
/// # Errors
/// - `dir` doesn't exist
/// - User doesn't have access to `dir`
///
/// Additionally, this skips any file/folder names
/// that has broken encoding (not UTF-8 or ASCII).
pub async fn read_filenames_from_dir_ext<P: AsRef<Path>>(dir: P) -> Result<Vec<DirItem>, IoError> {
    let dir: &Path = dir.as_ref();
    let mut entries = tokio::fs::read_dir(dir).await.dir(dir)?;
    let mut filenames = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(|n| IoError::ReadDir {
        error: n.to_string(),
        parent: dir.to_owned(),
    })? {
        if let Some(name) = entry.file_name().to_str() {
            filenames.push(DirItem {
                name: name.to_owned(),
                is_file: entry.path().is_file(),
            });
        }
    }

    Ok(filenames)
}

#[derive(Debug, Clone)]
pub struct DirItem {
    pub name: String,
    pub is_file: bool,
}

/// Finds the first in the specified directory
/// that matches the criteria specified by the
/// input function.
///
/// It reads the directory's entries, passing
/// the path and name to the input function.
/// If `true` is returned then that entry's path
/// will be returned, else it continues searching.
///
/// The order in which it searches is platform and filesystem
/// dependent, so essentially **non-deterministic**.
pub async fn find_item_in_dir<F: FnMut(&Path, &str) -> bool>(
    parent_dir: &Path,
    mut f: F,
) -> Result<Option<PathBuf>, IoError> {
    let mut entries = tokio::fs::read_dir(parent_dir).await.path(parent_dir)?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(OsStr::to_str) {
            if f(&path, file_name) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

pub async fn zip_directory_to_bytes<P: AsRef<Path>>(dir: P) -> std::io::Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);
    let options = FileOptions::<()>::default().unix_permissions(0o755);

    let dir = dir.as_ref();
    let base_path = dir;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(std::io::Error::other)?;
            let name_in_zip = relative_path.to_string_lossy().replace('\\', "/"); // For Windows compatibility

            zip.start_file(name_in_zip, options)?;
            let bytes = tokio::fs::read(path)
                .await
                .path(path)
                .map_err(std::io::Error::other)?;
            zip.write_all(&bytes)?;
        }
    }

    zip.finish()?;
    Ok(buffer.into_inner())
}
