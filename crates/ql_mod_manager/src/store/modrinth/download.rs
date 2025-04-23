use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use chrono::DateTime;
use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, InstanceSelection, IntoIoError,
};

use crate::store::{
    local_json::{ModConfig, ModIndex},
    modrinth::versions::ModVersion,
    ModError, SOURCE_ID_MODRINTH,
};

use super::info::ProjectInfo;

pub struct ModDownloader {
    version: String,
    pub index: ModIndex,
    loader: Option<String>,
    currently_installing_mods: HashSet<String>,
    pub info: HashMap<String, ProjectInfo>,
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
            info: HashMap::new(),
        })
    }

    pub async fn download_project(
        &mut self,
        id: &str,
        dependent: Option<&str>,
        manually_installed: bool,
    ) -> Result<(), ModError> {
        let project_info = if let Some(n) = self.info.get(id) {
            n.clone()
        } else {
            let info = ProjectInfo::download(id).await?;
            self.info.insert(id.to_owned(), info.clone());
            info
        };

        if self.is_already_installed(id, dependent, &project_info.title) {
            pt!("Already installed mod {id}, skipping.");
            return Ok(());
        }

        info!("Getting project info (id: {id})");

        if !self.has_compatible_loader(&project_info) {
            if let Some(loader) = &self.loader {
                pt!("Mod {} doesn't support {loader}", project_info.title);
            } else {
                err!("Mod {} doesn't support unknown loader!", project_info.title);
            }
            return Ok(());
        }

        print_downloading_message(&project_info, dependent);

        let download_version = self
            .get_download_version(id, project_info.title.clone())
            .await?;

        pt!("Getting dependencies");
        let mut dependency_list = HashSet::new();

        for dependency in &download_version.dependencies {
            let Some(ref dep_id) = dependency.project_id else {
                continue;
            };

            if dependency.dependency_type != "required" {
                pt!(
                    "Skipping dependency (not required: {}) {dep_id}",
                    dependency.dependency_type,
                );
                continue;
            }
            if dependency_list.insert(dep_id.clone()) {
                Box::pin(self.download_project(dep_id, Some(id), false)).await?;
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

    fn is_already_installed(&mut self, id: &str, dependent: Option<&str>, name: &str) -> bool {
        if let Some(mod_info) = self.index.mods.get_mut(id) {
            if let Some(dependent) = dependent {
                mod_info.dependents.insert(dependent.to_owned());
            } else {
                mod_info.manually_installed = true;
            }
            return true;
        }

        if let Some(mod_info) = self.index.mods.values_mut().find(|n| n.name == name) {
            if let Some(dependent) = dependent {
                mod_info.dependents.insert(dependent.to_owned());
            } else {
                mod_info.manually_installed = true;
            }
            return true;
        }

        !self.currently_installing_mods.insert(id.to_owned())
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

    async fn get_download_version(&self, id: &str, title: String) -> Result<ModVersion, ModError> {
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
            .next_back()
            .ok_or(ModError::NoCompatibleVersionFound(title))?;

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

async fn get_mods_dir(instance_name: &InstanceSelection) -> Result<PathBuf, ModError> {
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

pub async fn get_loader_type(instance: &InstanceSelection) -> Result<Option<String>, ModError> {
    let instance_dir = file_utils::get_instance_dir(instance).await?;
    let config_json = InstanceConfigJson::read_from_path(&instance_dir).await?;

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
