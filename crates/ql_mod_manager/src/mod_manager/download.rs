use std::{cmp::Ordering, collections::HashSet, path::PathBuf, sync::mpsc::Sender};

use chrono::DateTime;
use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoStringError,
};

use crate::rate_limiter::MOD_DOWNLOAD_LOCK;

use super::{ModConfig, ModError, ModIndex, ModVersion, ProjectInfo};

pub const SOURCE_ID_MODRINTH: &str = "modrinth";

pub async fn download_mods_w(
    ids: Vec<String>,
    instance_name: InstanceSelection,
    progress: Sender<GenericProgress>,
) -> Result<(), String> {
    let _guard = if let Ok(g) = MOD_DOWNLOAD_LOCK.try_lock() {
        g
    } else {
        info!("Another mod is already being installed... Waiting...");
        MOD_DOWNLOAD_LOCK.lock().await
    };

    let mut downloader = ModDownloader::new(&instance_name).await.strerr()?;

    let len = ids.len();
    for (i, id) in ids.iter().enumerate() {
        _ = progress.send(GenericProgress {
            done: i,
            total: len,
            message: None,
            has_finished: false,
        });
        pt!("Downloading: {} / {}", i + 1, len - 1);
        downloader.download_project(id, None, true).await.strerr()?;
    }

    info!("Finished installing {len} mods");

    downloader.index.save().await.strerr()?;

    Ok(())
}

pub async fn download_mod(id: &str, instance_name: &InstanceSelection) -> Result<(), ModError> {
    // Download one mod at a time
    let _guard = if let Ok(g) = MOD_DOWNLOAD_LOCK.try_lock() {
        g
    } else {
        info!("Another mod is already being installed... Waiting...");
        MOD_DOWNLOAD_LOCK.lock().await
    };

    let mut downloader = ModDownloader::new(instance_name).await?;

    downloader.download_project(id, None, true).await?;

    downloader.index.save().await?;

    pt!("Finished");

    Ok(())
}

pub async fn get_loader_type(instance: &InstanceSelection) -> Result<Option<String>, ModError> {
    let config_json = get_config_json(instance).await?;

    Ok(match config_json.mod_type.as_str() {
        "Fabric" => Some("fabric"),
        "Forge" => Some("forge"),
        "Quilt" => Some("quilt"),
        "NeoForge" => Some("neoforge"),
        "LiteLoader" => Some("liteloader"),
        "Rift" => Some("rift"),
        "OptiFine" => Some("optifine"),
        _ => {
            err!("Unknown loader {}", config_json.mod_type);
            None
        } // TODO: Add more loaders
    }
    .map(str::to_owned))
}

async fn get_config_json(instance: &InstanceSelection) -> Result<InstanceConfigJson, ModError> {
    let config_file_path = file_utils::get_instance_dir(instance)
        .await?
        .join("config.json");
    let config_json = tokio::fs::read_to_string(&config_file_path)
        .await
        .path(config_file_path)?;
    let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;
    Ok(config_json)
}

pub(crate) struct ModDownloader {
    version: String,
    pub index: ModIndex,
    loader: Option<String>,
    currently_installing_mods: HashSet<String>,
    mods_dir: PathBuf,
}

impl ModDownloader {
    pub async fn new(instance_name: &InstanceSelection) -> Result<ModDownloader, ModError> {
        let mods_dir = get_mods_dir(instance_name).await?;

        let version_json = VersionDetails::load(instance_name).await?;

        let index = ModIndex::get(instance_name).await?;
        let loader = get_loader_type(instance_name).await?;
        let currently_installing_mods = HashSet::new();
        Ok(ModDownloader {
            version: version_json.id,
            index,
            loader,
            currently_installing_mods,
            mods_dir,
        })
    }

    pub async fn download_project(
        &mut self,
        id: &str,
        dependent: Option<&str>,
        manually_installed: bool,
    ) -> Result<(), ModError> {
        info!("Getting project info (id: {id})");

        if self.is_already_installed(id, dependent) {
            pt!("Already installed mod {id}, skipping.");
            return Ok(());
        }

        let project_info = ProjectInfo::download(id.to_owned()).await?;

        if !self.has_compatible_loader(&project_info) {
            if let Some(loader) = &self.loader {
                pt!("Mod {} doesn't support {loader}", project_info.title);
            } else {
                err!("Mod {} doesn't support unknown loader!", project_info.title);
            }
            return Ok(());
        }

        print_downloading_message(&project_info, dependent);

        let download_version = self.get_download_version(id).await?;

        pt!("Getting dependencies");
        let mut dependency_list = HashSet::new();

        for dependency in &download_version.dependencies {
            if dependency.dependency_type != "required" {
                pt!(
                    "Skipping dependency (not required: {}) {}",
                    dependency.dependency_type,
                    dependency.project_id
                );
                continue;
            }
            if dependency_list.insert(dependency.project_id.clone()) {
                Box::pin(self.download_project(&dependency.project_id, Some(id), false)).await?;
            }
        }

        if !self.index.mods.contains_key(id) {
            self.download_file(&download_version).await?;
            add_mod_to_index(
                &mut self.index,
                id,
                &project_info,
                &download_version,
                dependency_list,
                dependent,
                manually_installed,
            );
        }

        Ok(())
    }

    fn is_already_installed(&mut self, id: &str, dependent: Option<&str>) -> bool {
        if let Some(mod_info) = self.index.mods.get_mut(id) {
            if let Some(dependent) = dependent {
                mod_info.dependents.insert(dependent.to_owned());
            }
        }
        !self.currently_installing_mods.insert(id.to_owned()) || self.index.mods.contains_key(id)
    }

    fn has_compatible_loader(&self, project_info: &ProjectInfo) -> bool {
        if let Some(loader) = &self.loader {
            if project_info.loaders.contains(loader) {
                true
            } else {
                pt!(
                    "Skipping mod {}: No compatible loader found",
                    project_info.title
                );
                false
            }
        } else {
            true
        }
    }

    async fn get_download_version(&self, id: &str) -> Result<ModVersion, ModError> {
        pt!("Getting download info");
        let download_info = ModVersion::download(id).await?;

        let mut download_versions: Vec<ModVersion> = download_info
            .iter()
            .filter(|v| v.game_versions.contains(&self.version))
            .filter(|v| {
                if let Some(loader) = &self.loader {
                    v.loaders.contains(loader)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by date published
        download_versions.sort_by(version_sort);

        let download_version = download_versions
            .into_iter()
            .last()
            .ok_or(ModError::NoCompatibleVersionFound)?;

        Ok(download_version)
    }

    async fn download_file(&self, download_version: &ModVersion) -> Result<(), ModError> {
        if let Some(primary_file) = download_version.files.iter().find(|file| file.primary) {
            let file_bytes = file_utils::download_file_to_bytes(&primary_file.url, true).await?;
            let file_path = self.mods_dir.join(&primary_file.filename);
            tokio::fs::write(&file_path, &file_bytes)
                .await
                .path(file_path)?;
        } else {
            pt!("Didn't find primary file, checking secondary files...");
            for file in &download_version.files {
                let file_bytes = file_utils::download_file_to_bytes(&file.url, true).await?;
                let file_path = self.mods_dir.join(&file.filename);
                tokio::fs::write(&file_path, &file_bytes)
                    .await
                    .path(file_path)?;
            }
        }
        Ok(())
    }
}

pub async fn get_mods_dir(instance_name: &InstanceSelection) -> Result<PathBuf, ModError> {
    let dot_minecraft_dir = file_utils::get_dot_minecraft_dir(instance_name).await?;
    let mods_dir = dot_minecraft_dir.join("mods");
    if !mods_dir.exists() {
        tokio::fs::create_dir(&mods_dir).await.path(&mods_dir)?;
    }
    Ok(mods_dir)
}

pub fn version_sort(a: &ModVersion, b: &ModVersion) -> Ordering {
    let a = &a.date_published;
    let b = &b.date_published;
    let a = match DateTime::parse_from_rfc3339(a) {
        Ok(date) => date,
        Err(err) => {
            err!("Couldn't parse date {a}: {err}");
            return Ordering::Equal;
        }
    };

    let b = match DateTime::parse_from_rfc3339(b) {
        Ok(date) => date,
        Err(err) => {
            err!("Couldn't parse date {b}: {err}");
            return Ordering::Equal;
        }
    };

    a.cmp(&b)
}

fn add_mod_to_index(
    index: &mut ModIndex,
    id: &str,
    project_info: &ProjectInfo,
    download_version: &ModVersion,
    dependency_list: HashSet<String>,
    dependent: Option<&str>,
    manually_installed: bool,
) {
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
            manually_installed,
            enabled: true,
            installed_version: download_version.version_number.clone(),
            version_release_time: download_version.date_published.clone(),
            project_source: SOURCE_ID_MODRINTH.to_owned(),
        },
    );
}

fn print_downloading_message(project_info: &ProjectInfo, dependent: Option<&str>) {
    if let Some(dependent) = dependent {
        pt!(
            "Downloading {}: Dependency of {dependent}",
            project_info.title
        );
    } else {
        pt!("Downloading {}", project_info.title);
    }
}
