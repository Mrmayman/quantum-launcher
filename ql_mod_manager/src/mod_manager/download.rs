use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use chrono::DateTime;
use ql_instances::{
    err, file_utils, info, io_err,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    pt, MOD_DOWNLOAD_LOCK,
};
use reqwest::Client;

use super::{ModConfig, ModError, ModIndex, ModVersion, ProjectInfo};

pub async fn download_mod_wrapped(id: String, instance_name: String) -> Result<String, String> {
    download_mod(id, instance_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn download_mod(id: String, instance_name: String) -> Result<String, ModError> {
    // Download one mod at a time
    let _guard = if let Ok(g) = MOD_DOWNLOAD_LOCK.try_lock() {
        g
    } else {
        info!("Another mod is already being installed... Waiting...");
        MOD_DOWNLOAD_LOCK.lock().await
    };

    let mut downloader = ModDownloader::new(&instance_name)?;

    downloader.download_project(&id, None, true).await?;

    downloader.index.save()?;

    pt!("Finished");

    Ok(id)
}

pub fn get_loader_type(instance_dir: &Path) -> Result<Option<String>, ModError> {
    let config_json = get_config_json(instance_dir)?;

    Ok(match config_json.mod_type.as_str() {
        "Fabric" => Some("fabric"),
        "Forge" => Some("forge"),
        _ => {
            err!("Unknown loader {}", config_json.mod_type);
            None
        } // TODO: Add more loaders
    }
    .map(str::to_owned))
}

pub fn get_instance_and_mod_dir(instance_name: &str) -> Result<(PathBuf, PathBuf), ModError> {
    let instance_dir = file_utils::get_launcher_dir()?
        .join("instances")
        .join(instance_name);
    let mods_dir = instance_dir.join(".minecraft/mods");
    if !mods_dir.exists() {
        std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
    }
    Ok((instance_dir, mods_dir))
}

pub fn get_version_json(instance_dir: &Path) -> Result<VersionDetails, ModError> {
    let version_json_path = instance_dir.join("details.json");
    let version_json: String =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;
    Ok(version_json)
}

fn get_config_json(instance_dir: &Path) -> Result<InstanceConfigJson, ModError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json =
        std::fs::read_to_string(&config_file_path).map_err(io_err!(config_file_path))?;
    let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;
    Ok(config_json)
}

struct ModDownloader {
    version: String,
    index: ModIndex,
    loader: Option<String>,
    currently_installing_mods: HashSet<String>,
    client: Client,
    mods_dir: PathBuf,
}

impl ModDownloader {
    fn new(instance_name: &str) -> Result<ModDownloader, ModError> {
        let (instance_dir, mods_dir) = get_instance_and_mod_dir(instance_name)?;
        let version_json = get_version_json(&instance_dir)?;
        let index = ModIndex::get(instance_name)?;
        let client = reqwest::Client::new();
        let loader = get_loader_type(&instance_dir)?;
        let currently_installing_mods = HashSet::new();
        Ok(ModDownloader {
            version: version_json.id,
            index,
            loader,
            currently_installing_mods,
            client,
            mods_dir,
        })
    }

    #[async_recursion]
    async fn download_project(
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

        // pt!("Getting dependencies");
        // let dependencies = Dependencies::download(id).await?;
        let dependency_list = HashSet::new();

        /*for dependency in &dependencies.projects {
            if !dependency.game_versions.contains(&self.version) {
                eprintln!(
                    "[warn] Dependency {} doesn't support version {}",
                    dependency.title, self.version
                );
                continue;
            }

            if let Some(loader) = &self.loader {
                if !dependency.loaders.contains(loader) {
                    eprintln!(
                        "[warn] Dependency {} doesn't support loader {loader}",
                        dependency.title
                    );
                    continue;
                }
            }

            self.download_project(&dependency.id, Some(id), false)
                .await?;
            dependency_list.insert(dependency.id.clone());
        }*/

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
                println!(
                    "- Skipping mod {}: No compatible loader found",
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
            let file_bytes =
                file_utils::download_file_to_bytes(&self.client, &primary_file.url, true).await?;
            let file_path = self.mods_dir.join(&primary_file.filename);
            std::fs::write(&file_path, &file_bytes).map_err(io_err!(file_path))?;
        } else {
            pt!("Didn't find primary file, checking secondary files...");
            for file in &download_version.files {
                let file_bytes =
                    file_utils::download_file_to_bytes(&self.client, &file.url, true).await?;
                let file_path = self.mods_dir.join(&file.filename);
                std::fs::write(&file_path, &file_bytes).map_err(io_err!(file_path))?;
            }
        }
        Ok(())
    }
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
