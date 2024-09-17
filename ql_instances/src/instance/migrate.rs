use std::path::Path;

use crate::{
    download::GameDownloader,
    error::LauncherError,
    info, io_err,
    json_structs::json_version::{LibraryDownloads, VersionDetails},
    LAUNCHER_VERSION,
};

pub async fn migrate_old_instances(instance_dir: &Path) -> Result<(), LauncherError> {
    let launcher_version_path = instance_dir.join("launcher_version.txt");
    let mut version = if !launcher_version_path.exists() {
        std::fs::write(&launcher_version_path, "0.1").map_err(io_err!(launcher_version_path))?;
        "0.1".to_owned()
    } else {
        std::fs::read_to_string(&launcher_version_path).map_err(io_err!(launcher_version_path))?
    };
    if version.split('.').count() == 2 {
        version.push_str(".0");
    }

    let version = semver::Version::parse(&version)?;

    let client = reqwest::Client::new();
    if version < LAUNCHER_VERSION {
        migrate_download_missing_native_libs(instance_dir, &client).await?;
    }

    Ok(())
}

async fn migrate_download_missing_native_libs(
    instance_dir: &Path,
    client: &reqwest::Client,
) -> Result<(), LauncherError> {
    let json_path = instance_dir.join("details.json");
    let json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
    let json: VersionDetails = serde_json::from_str(&json)?;

    info!("Downloading missing native libraries");
    let bar = indicatif::ProgressBar::new(json.libraries.len() as u64);

    for library in json.libraries.iter() {
        if !GameDownloader::download_libraries_library_is_allowed(library) {
            continue;
        }

        if let Some(LibraryDownloads::Normal { artifact, .. }) = &library.downloads {
            let library_path = instance_dir.join("libraries").join(&artifact.path);
            let library_file = std::fs::read(&library_path).map_err(io_err!(library_path))?;

            GameDownloader::extract_native_library(
                instance_dir,
                client,
                library,
                &library_file,
                artifact,
                &bar,
            )
            .await?;
        }
    }

    Ok(())
}
