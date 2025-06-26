use std::{collections::HashMap, path::Path, sync::mpsc::Sender};

use ql_core::{
    do_jobs, file_utils,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection,
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

                let mut download = download.clone();

                // Known broken mods, included in Re-Console modpack
                // https://modrinth.com/modpack/legacy-minecraft
                // These fix the crash, but I still get a black screen
                if download == "https://cdn.modrinth.com/data/u58R1TMW/versions/WFiIDhbD/connector-2.0.0-beta.2%2B1.21.1-full.jar" {
                    "https://cdn.modrinth.com/data/u58R1TMW/versions/k3UrqfQk/connector-2.0.0-beta.6%2B1.21.1-full.jar".clone_into(&mut download);
                } else if download == "https://cdn.modrinth.com/data/gHvKJofA/versions/GvTZJhPo/Legacy4J-1.21-1.7.2-neoforge.jar"
                    || download == "https://cdn.modrinth.com/data/gHvKJofA/versions/fYlGcfZd/Legacy4J-1.21-1.7.3-neoforge.jar" {
                    "https://cdn.modrinth.com/data/gHvKJofA/versions/RD8XgI0Y/Legacy4J-1.21-1.7.4-neoforge.jar".clone_into(&mut download);
                }

                let bytes_path = mc_dir.join(&file.path);
                file_utils::download_file_to_path(&download, true, &bytes_path).await?;

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
                .map(str::to_owned)
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
