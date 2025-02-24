use std::path::{Path, PathBuf};

use crate::{file_utils, IntoIoError, IoError, JsonFileError};
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct JsonOptifine {
    pub id: String,
    pub libraries: Vec<OptifineLibrary>,
    pub mainClass: String,
    pub arguments: Option<OptifineArguments>,
    pub minecraftArguments: Option<String>,
}

impl JsonOptifine {
    /// Reads the OptiFine JSON file and JAR file from the instance directory.
    ///
    /// This function takes the name of the instance and looks for the OptiFine
    /// directory in the .minecraft/versions directory.
    ///
    /// It returns the parsed JSON file and the path to the JAR file.
    ///
    /// # Errors
    /// - If the versions dir does not exist:
    ///   `QuantumLauncher/instances/<instance_name>/.minecraft/versions/`
    /// - If any directory starting with "Opti" is not found in the versions dir
    /// - If the OptiFine directory does not contain a JSON file or JAR file
    /// - If the config directory (`AppData/Roaming` or `~/.config`) does not exist
    pub async fn read(instance_name: &str) -> Result<(Self, PathBuf), JsonFileError> {
        let dot_minecraft_dir = file_utils::get_launcher_dir()
            .await?
            .join("instances")
            .join(instance_name)
            .join(".minecraft/versions");

        let optifine_version_dir = find_subdirectory_with_name(&dot_minecraft_dir, "Opti")
            .await?
            .ok_or(IoError::Io {
                error: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find OptiFine directory",
                ),
                path: dot_minecraft_dir.clone(),
            })?;

        let (json, jar) = find_and_read_json_with_jar(&optifine_version_dir).await?;

        Ok((serde_json::from_str::<Self>(&json)?, jar))
    }
}

async fn find_and_read_json_with_jar(dir_path: &Path) -> Result<(String, PathBuf), IoError> {
    // Read the directory entries
    let mut entries = tokio::fs::read_dir(dir_path).await.path(dir_path)?;

    let mut json_content: Option<String> = None;
    let mut jar_path: Option<PathBuf> = None;

    // Iterate over the directory entries
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        // Check if the entry is a file
        if !path.is_file() {
            continue;
        }
        if let Some(extension) = path.extension() {
            if extension == "json" {
                // Read the contents of the JSON file
                let contents = tokio::fs::read_to_string(&path).await.path(path)?;
                json_content = Some(contents);
            } else if extension == "jar" {
                // Record the path to the .jar file
                jar_path = Some(path);
            }
        }

        // If both the JSON content and JAR path are found, break early
        if json_content.is_some() && jar_path.is_some() {
            break;
        }
    }

    // Ensure both JSON content and JAR path are found
    let json_content = json_content.ok_or({
        IoError::Io {
            error: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No JSON file found in the directory",
            ),
            path: dir_path.to_owned(),
        }
    })?;

    let jar_path = jar_path.ok_or({
        IoError::Io {
            error: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Jar file found in the directory",
            ),
            path: dir_path.to_owned(),
        }
    })?;

    Ok((json_content, jar_path))
}

async fn find_subdirectory_with_name(
    parent_dir: &Path,
    keyword: &str,
) -> Result<Option<PathBuf>, IoError> {
    // Read the contents of the directory
    let mut entries = tokio::fs::read_dir(parent_dir).await.path(parent_dir)?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        // Check if the entry is a directory and contains the keyword
        if !path.is_dir() {
            continue;
        }

        if let Some(Some(file_name)) = path.file_name().map(|n| n.to_str()) {
            if file_name.to_lowercase().contains(&keyword.to_lowercase()) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

#[derive(Deserialize)]
pub struct OptifineLibrary {
    pub name: String,
}

#[derive(Deserialize)]
pub struct OptifineArguments {
    pub game: Vec<String>,
}
