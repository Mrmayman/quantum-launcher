use std::{fs, path::PathBuf};

use reqwest::blocking::Client;

use crate::error::{LauncherError, LauncherResult};

pub fn get_launcher_dir() -> LauncherResult<PathBuf> {
    let config_directory = match dirs::config_dir() {
        Some(d) => d,
        None => return Err(LauncherError::ConfigDirNotFound),
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    create_dir_if_not_exists(&launcher_directory)?;

    Ok(launcher_directory)
}

pub fn create_dir_if_not_exists(path: &PathBuf) -> LauncherResult<()> {
    if !path.exists() {
        match fs::create_dir_all(&path) {
            Ok(_) => Ok(()),
            Err(err) => Err(LauncherError::IoError(err)),
        }
    } else {
        Ok(())
    }
}

pub fn download_file(client: &Client, url: &str) -> LauncherResult<String> {
    let response = client.get(url).send()?;
    if response.status().is_success() {
        Ok(response.text()?)
    } else {
        Err(LauncherError::ReqwestStatusError(response.status()))
    }
}
