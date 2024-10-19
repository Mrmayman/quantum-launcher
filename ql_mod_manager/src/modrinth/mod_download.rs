use std::{collections::HashSet, path::Path};

use async_recursion::async_recursion;
use ql_instances::{
    error::IoError,
    file_utils, info, io_err,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    MOD_DOWNLOAD_LOCK,
};

use super::{
    get_project::Dependencies, ModConfig, ModIndex, ModVersion, ModrinthError, ProjectInfo,
};

pub async fn download_mod_wrapped(id: String, instance_name: String) -> Result<(), String> {
    download_mod(&id, instance_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn download_mod(id: &str, instance_name: String) -> Result<(), ModrinthError> {
    // Download one mod at a time
    let _guard = if let Ok(g) = MOD_DOWNLOAD_LOCK.try_lock() {
        g
    } else {
        info!("Another mod is already being installed... Waiting...");
        MOD_DOWNLOAD_LOCK.lock().await
    };

    let instance_dir = file_utils::get_launcher_dir()?
        .join("instances")
        .join(&instance_name);

    let mods_dir = instance_dir.join(".minecraft/mods");
    if !mods_dir.exists() {
        std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
    }

    let version_json_path = instance_dir.join("details.json");

    let version_json: String =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let client = reqwest::Client::new();

    let mut index = ModIndex::get(&instance_name)?;

    let config_file_path = instance_dir.join("config.json");
    let config_json =
        std::fs::read_to_string(&config_file_path).map_err(io_err!(config_file_path))?;
    let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

    let loader = match config_json.mod_type.as_str() {
        "Fabric" => Some("fabric"),
        "Forge" => Some("forge"),
        _ => None,
        // TODO: Add more loaders
    }
    .map(str::to_owned);

    let mut currently_installing_mods = HashSet::new();

    download_project(
        id,
        &version_json.id,
        None,
        &mut index,
        &client,
        &mods_dir,
        loader.as_ref(),
        &mut currently_installing_mods,
    )
    .await?;

    index.save()?;

    info!("Finished installing mod");

    Ok(())
}

#[async_recursion]
async fn download_project(
    id: &str,
    version: &String,
    dependent: Option<&str>,
    index: &mut ModIndex,
    client: &reqwest::Client,
    mods_dir: &Path,
    loader: Option<&String>,
    currently_installing_mods: &mut HashSet<String>,
) -> Result<(), ModrinthError> {
    info!("Getting project info (id: {id})");
    if !currently_installing_mods.insert(id.to_owned()) {
        info!("Already installed mod {id}, skipping.");
        return Ok(());
    }
    let project_info = ProjectInfo::download(id.to_owned()).await?;

    if let Some(loader) = loader {
        if !project_info.loaders.contains(loader) {
            info!(
                "Skipping mod {}: No compatible loader found",
                project_info.title
            );
            return Ok(());
        }
    }

    if let Some(dependent) = dependent {
        info!(
            "Downloading {}: Dependency of {dependent}",
            project_info.title
        );
    } else {
        info!("Downloading {}", project_info.title);
    }
    info!("Getting download info");
    let download_info = ModVersion::download(id).await?;

    let download_version = download_info
        .iter()
        .filter(|v| v.game_versions.contains(version))
        .find(|v| {
            if let Some(loader) = loader {
                v.loaders.contains(loader)
            } else {
                true
            }
        })
        .ok_or(ModrinthError::NoCompatibleVersionFound)?;

    info!("Getting dependencies");
    let dependencies = Dependencies::download(id).await?;

    let mut dependency_list = HashSet::new();

    for file in &dependencies.projects {
        if !file.game_versions.contains(version) {
            eprintln!("[warn] Dependency {} does not support version {version}\n- Supported versions: {:?}", file.title, file.game_versions);
            continue;
        }

        if let Some(loader) = loader {
            if !file.loaders.contains(loader) {
                eprintln!("[warn] Dependency {} does not support version {version}\n- Supported versions: {:?}", file.title, file.game_versions);
                continue;
            }
        }

        download_project(
            &file.id,
            version,
            Some(id),
            index,
            client,
            mods_dir,
            loader,
            currently_installing_mods,
        )
        .await?;
        dependency_list.insert(file.id.clone());
    }

    if let Some(mod_info) = index.mods.get_mut(id) {
        if let Some(dependent) = dependent {
            mod_info.dependents.insert(dependent.to_owned());
        }
    } else {
        if let Some(primary_file) = download_version.files.iter().find(|file| file.primary) {
            let file_bytes =
                file_utils::download_file_to_bytes(client, &primary_file.url, true).await?;
            let file_path = mods_dir.join(&primary_file.filename);
            std::fs::write(&file_path, &file_bytes).map_err(io_err!(file_path))?;
        } else {
            info!("Didn't find primary file, checking secondary files...");
            for file in &download_version.files {
                let file_bytes =
                    file_utils::download_file_to_bytes(client, &file.url, true).await?;
                let file_path = mods_dir.join(&file.filename);
                std::fs::write(&file_path, &file_bytes).map_err(io_err!(file_path))?;
            }
        }

        index.mods.insert(
            id.to_owned(),
            ModConfig {
                name: project_info.title.clone(),
                description: project_info.description.clone(),
                icon_url: project_info.icon_url.clone(),
                project_id: id.to_owned(),
                files: download_version.files.clone(),
                supported_versions: download_version.game_versions.clone(),
                dependencies: dependency_list,
                dependents: if let Some(dependent) = dependent {
                    let mut set = HashSet::new();
                    set.insert(dependent.to_owned());
                    set
                } else {
                    HashSet::new()
                },
            },
        );
    }

    Ok(())
}

pub async fn delete_mod_wrapped(id: String, instance_name: String) -> Result<String, String> {
    delete_mod(&id, instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| id)
}

pub async fn delete_mod(id: &str, instance_name: String) -> Result<(), ModrinthError> {
    let mut index = ModIndex::get(&instance_name)?;

    let launcher_dir = file_utils::get_launcher_dir()?;
    let mods_dir = launcher_dir
        .join("instances")
        .join(&instance_name)
        .join(".minecraft/mods");
    delete_item(id, None, &mut index, &mods_dir)?;

    index.save()?;
    Ok(())
}

fn delete_item(
    id: &str,
    parent: Option<&str>,
    index: &mut ModIndex,
    mods_dir: &Path,
) -> Result<(), ModrinthError> {
    info!("Deleting mod {id}");
    if let Some(mod_info) = index.mods.get_mut(id) {
        if let Some(parent) = parent {
            mod_info.dependencies = mod_info
                .dependencies
                .iter()
                .filter_map(|n| {
                    if n.as_str() == parent {
                        None
                    } else {
                        Some(n.clone())
                    }
                })
                .collect();

            if !mod_info.dependencies.is_empty() {
                return Ok(());
            }
        }
    } else {
        eprintln!("[error] Could not find mod in index: {id}");
    }
    if let Some(mod_info) = index.mods.get(id).cloned() {
        for file in &mod_info.files {
            let path = mods_dir.join(&file.filename);
            if let Err(err) = std::fs::remove_file(&path) {
                if let std::io::ErrorKind::NotFound = err.kind() {
                    eprintln!("[warning] File does not exist, skipping: {path:?}");
                } else {
                    let err = IoError::Io {
                        error: err,
                        path: path.to_owned(),
                    };
                    Err(err)?;
                }
            }
        }

        for dependency in &mod_info.dependencies {
            delete_item(dependency, Some(id), index, mods_dir)?;
        }
    }
    index.mods.remove(id);
    Ok(())
}
