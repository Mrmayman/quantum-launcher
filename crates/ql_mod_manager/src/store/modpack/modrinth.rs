use std::{collections::HashMap, path::Path, sync::mpsc::Sender};

use ql_core::{
    do_jobs, file_utils,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use super::PackError;

#[derive(Deserialize)]
pub struct PackIndex {
    pub name: String,
    pub files: Vec<PackFile>,

    /// Info about which Minecraft version
    /// and Loader version is required. May contain:
    ///
    /// - `minecraft` (always present)
    /// - `forge`
    /// - `neoforge`
    /// - `fabric-loader`
    /// - `quilt-loader`
    pub dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct PackFile {
    pub path: String,
    pub env: PackEnv,
    pub downloads: Vec<String>,
}

#[derive(Deserialize)]
pub struct PackEnv {
    pub client: String,
    pub server: String,
}

pub async fn install(
    instance: &InstanceSelection,
    mc_dir: &Path,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &PackIndex,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<(), PackError> {
    if let Some(version) = index.dependencies.get("minecraft") {
        if json.id != *version {
            return Err(PackError::GameVersion {
                expect: version.clone(),
                got: json.id.clone(),
            });
        }
    }

    pt!("Modrinth Modpack: {}", index.name);
    let loader = match config.mod_type.as_str() {
        "Forge" => "forge",
        "Fabric" => "fabric-loader",
        "Quilt" => "quilt-loader",
        "NeoForge" => "neoforge",
        _ => {
            return Err(expect_got_modrinth(index, config));
        }
    };
    if !index.dependencies.contains_key(loader) {
        return Err(expect_got_modrinth(index, config));
    }

    let i = Mutex::new(0);
    let i = &i;

    let len = index.files.len();
    let jobs: Result<Vec<()>, PackError> = do_jobs(
        index
            .files
            .iter()
            .filter_map(|file| file.downloads.first().map(|n| (file, n)))
            .map(|(file, download)| async move {
                let required_field = match instance {
                    InstanceSelection::Instance(_) => &file.env.client,
                    InstanceSelection::Server(_) => &file.env.server,
                };
                if required_field != "required" {
                    pt!("Skipping {} (optional)", file.path);
                    return Ok(());
                }

                let bytes = file_utils::download_file_to_bytes(download, true).await?;
                let bytes_path = mc_dir.join(&file.path);
                tokio::fs::write(&bytes_path, &bytes)
                    .await
                    .path(bytes_path)?;

                if let Some(sender) = sender {
                    let mut i = i.lock().await;
                    _ = sender.send(GenericProgress {
                        done: *i,
                        total: len,
                        message: Some(format!(
                            "Modpack: Installed mod (modrinth) ({i}/{len}):\n{}",
                            file.path,
                            i = *i + 1
                        )),
                        has_finished: false,
                    });
                    pt!(
                        "Installed mod (modrinth) ({i}/{len}): {}",
                        file.path,
                        i = *i + 1,
                    );
                    *i += 1;
                }

                Ok(())
            }),
    )
    .await;
    jobs?;

    Ok(())
}

fn expect_got_modrinth(index_json: &PackIndex, config: &InstanceConfigJson) -> PackError {
    match index_json
        .dependencies
        .iter()
        .filter_map(|(k, _)| (k != "minecraft").then_some(k.clone()))
        .map(|loader| {
            loader
                .strip_suffix("-loader")
                .map(|n| n.to_owned())
                .unwrap_or(loader)
        })
        .next()
    {
        Some(expect) => PackError::Loader {
            expect,
            got: config.mod_type.clone(),
        },
        None => PackError::NoLoadersSpecified,
    }
}
