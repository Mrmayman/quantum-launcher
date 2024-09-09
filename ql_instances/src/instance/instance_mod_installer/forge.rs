use std::num::ParseIntError;

use crate::{
    error::IoError,
    file_utils::{self, RequestError},
    io_err,
    json_structs::{
        json_forge::JsonForgeVersions, json_version::VersionDetails, JsonDownloadError,
    },
};

pub async fn install(instance_name: &str) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let lock_path = instance_dir.join("forge.lock");
    std::fs::write(
        &lock_path,
        "If you see this, forge was not installed correctly.",
    )
    .map_err(io_err!(lock_path))?;

    let version_json_path = instance_dir.join("details.json");
    let version_json =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json = serde_json::from_str::<VersionDetails>(&version_json)?;

    let minecraft_version = version_json.id;
    let forge_versions_json = JsonForgeVersions::download().await?;
    let forge_version = forge_versions_json
        .get_forge_version(&minecraft_version)
        .ok_or(ForgeInstallError::NoForgeVersionFound)?;

    // 1.20 -> 1.20.0
    let norm_version = {
        let number_of_full_stops = minecraft_version.chars().filter(|c| *c == '.').count();
        if number_of_full_stops == 1 {
            format!("{minecraft_version}.0")
        } else {
            minecraft_version.to_owned()
        }
    };

    let short_forge_version = format!("{minecraft_version}-{forge_version}");
    let norm_forge_version = format!("{short_forge_version}-{norm_version}");

    let forge_major_version: usize = forge_version
        .split('.')
        .next()
        .unwrap_or(&forge_version)
        .parse()?;

    let forge_dir = instance_dir.join("forge");
    std::fs::create_dir_all(&forge_dir).map_err(io_err!(forge_dir))?;

    let client = reqwest::Client::new();

    let file_type = if forge_major_version < 27 {
        "universal"
    } else {
        "installer"
    };

    let url = format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}-{file_type}.jar");
    let file = match file_utils::download_file_to_bytes(&client, &url).await {
        Ok(file) => file,
        Err(_) => {
            let url = format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{norm_forge_version}/forge-{norm_forge_version}-{file_type}.jar");
            file_utils::download_file_to_bytes(&client, &url).await?
        }
    };
    let file_name = format!("forge-{short_forge_version}-{file_type}.jar");
    let file_path = forge_dir.join(&file_name);
    std::fs::write(&file_path, &file).map_err(io_err!(file_path))?;

    let libraries_dir = forge_dir.join("libraries");
    std::fs::create_dir_all(&libraries_dir).map_err(io_err!(libraries_dir))?;

    todo!()
}

#[derive(Debug)]
pub enum ForgeInstallError {
    Io(IoError),
    Request(RequestError),
    Serde(serde_json::Error),
    NoForgeVersionFound,
    ParseIntError(ParseIntError),
}

impl From<IoError> for ForgeInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<RequestError> for ForgeInstallError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<serde_json::Error> for ForgeInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<ParseIntError> for ForgeInstallError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl From<JsonDownloadError> for ForgeInstallError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => Self::Request(err),
            JsonDownloadError::SerdeError(err) => Self::Serde(err),
        }
    }
}
