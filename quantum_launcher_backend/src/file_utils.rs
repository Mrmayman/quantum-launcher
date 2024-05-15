use std::path::PathBuf;

use reqwest::Client;

use crate::error::{LauncherError, LauncherResult};

pub fn get_launcher_dir() -> LauncherResult<PathBuf> {
    let config_directory = match dirs::config_dir() {
        Some(d) => d,
        None => return Err(LauncherError::ConfigDirNotFound),
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    std::fs::create_dir_all(&launcher_directory)
        .map_err(|err| LauncherError::IoError(err, launcher_directory.clone()))?;

    Ok(launcher_directory)
}

pub async fn download_file_to_string(client: &Client, url: &str) -> Result<String, RequestError> {
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        })
    }
}

pub async fn download_file_to_bytes(client: &Client, url: &str) -> Result<Vec<u8>, RequestError> {
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        })
    }
}

#[derive(Debug)]
pub enum RequestError {
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for RequestError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}
