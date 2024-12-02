use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{error::IoError, file_utils, io_err};

use super::JsonFileError;

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct JsonOptifine {
    pub id: String,
    pub libraries: Vec<OptifineLibrary>,
    pub mainClass: String,
    pub arguments: Option<OptifineArguments>,
    pub minecraftArguments: Option<String>,
}

impl JsonOptifine {
    pub fn read(instance_name: &str) -> Result<(Self, PathBuf), JsonFileError> {
        let dot_minecraft_dir = file_utils::get_launcher_dir()?
            .join("instances")
            .join(&instance_name)
            .join(".minecraft/versions");

        let optifine_version_dir =
            find_subdirectory_with_name(&dot_minecraft_dir, "Opti")?.ok_or(IoError::Io {
                error: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not find OptiFine directory",
                ),
                path: dot_minecraft_dir.to_owned(),
            })?;

        let (json, jar) = find_and_read_json_with_jar(&optifine_version_dir)?;

        Ok((serde_json::from_str::<Self>(&json)?, jar))
    }
}

fn find_and_read_json_with_jar(dir_path: &Path) -> Result<(String, PathBuf), IoError> {
    // Read the directory entries
    let entries = std::fs::read_dir(dir_path).map_err(io_err!(dir_path))?;

    let mut json_content: Option<String> = None;
    let mut jar_path: Option<PathBuf> = None;

    // Iterate over the directory entries
    for entry in entries {
        let entry = entry.map_err(io_err!(dir_path))?; // Handle possible errors
        let path = entry.path();

        // Check if the entry is a file
        if !path.is_file() {
            continue;
        }
        if let Some(extension) = path.extension() {
            if extension == "json" {
                // Read the contents of the JSON file
                let contents = std::fs::read_to_string(&path).map_err(io_err!(path))?;
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

fn find_subdirectory_with_name(
    parent_dir: &Path,
    keyword: &str,
) -> Result<Option<PathBuf>, IoError> {
    // Read the contents of the directory
    let entries = std::fs::read_dir(parent_dir).map_err(io_err!(parent_dir))?;
    for entry in entries.into_iter().filter_map(Result::ok) {
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

#[derive(Serialize, Deserialize)]
pub struct OptifineLibrary {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct OptifineArguments {
    pub game: Vec<String>,
}
