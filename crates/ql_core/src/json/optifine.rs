use std::path::{Path, PathBuf};

use crate::{
    file_utils::find_item_in_dir, IntoIoError, IntoJsonError, IoError, JsonFileError, LAUNCHER_DIR,
};
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
    /// Reads the Optifine JSON file and JAR file from the instance directory.
    ///
    /// This function takes the name of the instance and looks for the Optifine
    /// directory in the .minecraft/versions directory.
    ///
    /// It returns the parsed JSON file and the path to the JAR file.
    ///
    /// # Errors
    /// - If the versions dir does not exist:
    ///   `QuantumLauncher/instances/<instance_name>/.minecraft/versions/`
    /// - If any directory starting with "Opti" is not found in the versions dir
    /// - If the Optifine directory does not contain a JSON file or JAR file
    /// - If the config directory (`AppData/Roaming` or `~/.config`) does not exist
    pub async fn read(instance_name: &str) -> Result<(Self, PathBuf), JsonFileError> {
        let dot_minecraft_dir = LAUNCHER_DIR
            .join("instances")
            .join(instance_name)
            .join(".minecraft/versions");

        let optifine_version_dir = find_subdirectory_with_name(&dot_minecraft_dir, "Opti")
            .await?
            .ok_or(IoError::Io {
                error: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find OptiFine directory",
                )
                .to_string(),
                path: dot_minecraft_dir.clone(),
            })?;

        let (json, jar) = find_and_read_json_with_jar(&optifine_version_dir).await?;

        Ok((serde_json::from_str::<Self>(&json).json(json)?, jar))
    }
}

async fn find_and_read_json_with_jar(dir_path: &Path) -> Result<(String, PathBuf), IoError> {
    let mut entries = tokio::fs::read_dir(dir_path).await.path(dir_path)?;

    let mut json_content: Option<String> = None;
    let mut jar_path: Option<PathBuf> = None;

    // Scans through the directory, looking for both a Jar and JSON file
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }
        if let Some(extension) = path.extension() {
            if extension == "json" {
                let contents = tokio::fs::read_to_string(&path).await.path(path)?;
                json_content = Some(contents);
            } else if extension == "jar" {
                jar_path = Some(path);
            }
        }

        // Make sure we read both JSON and Jar
        if json_content.is_some() && jar_path.is_some() {
            break;
        }
    }

    // Ehh... should I brutalize std::io::Error like this?
    let json_content = json_content.ok_or({
        IoError::Io {
            error: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No JSON file found in the directory",
            )
            .to_string(),
            path: dir_path.to_owned(),
        }
    })?;

    let jar_path = jar_path.ok_or({
        IoError::Io {
            error: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Jar file found in the directory",
            )
            .to_string(),
            path: dir_path.to_owned(),
        }
    })?;

    Ok((json_content, jar_path))
}

async fn find_subdirectory_with_name(
    parent_dir: &Path,
    keyword: &str,
) -> Result<Option<PathBuf>, IoError> {
    find_item_in_dir(parent_dir, |path, name| {
        path.is_dir() && name.to_lowercase().contains(&keyword.to_lowercase())
    })
    .await
}

#[derive(Deserialize)]
pub struct OptifineLibrary {
    pub name: String,
}

#[derive(Deserialize)]
pub struct OptifineArguments {
    pub game: Vec<String>,
}
