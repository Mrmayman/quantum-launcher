use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use chrono::DateTime;
use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection,
};

use crate::store::{
    get_mods_resourcepacks_shaderpacks_dir, install_modpack,
    local_json::{ModConfig, ModIndex},
    modrinth::versions::ModVersion,
    CurseforgeNotAllowed, ModError, QueryType, SOURCE_ID_MODRINTH,
};

use super::info::ProjectInfo;

pub struct ModDownloader {
    version: String,
    pub index: ModIndex,
    loader: Option<String>,
    currently_installing_mods: HashSet<String>,
    pub info: HashMap<String, ProjectInfo>,
    instance: InstanceSelection,
    sender: Option<Sender<GenericProgress>>,

    mods_dir: PathBuf,
    resourcepacks_dir: PathBuf,
    shaderpacks_dir: PathBuf,
}

impl ModDownloader {
    pub async fn new(
        instance: &InstanceSelection,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<ModDownloader, ModError> {
        let version_json = VersionDetails::load(instance).await?;
        let (mods_dir, resourcepacks_dir, shaderpacks_dir) =
            get_mods_resourcepacks_shaderpacks_dir(instance, &version_json).await?;

        let index = ModIndex::get(instance).await?;
        let loader = get_loader_type(instance).await?;
        let currently_installing_mods = HashSet::new();
        Ok(ModDownloader {
            version: version_json.id,
            index,
            loader,
            currently_installing_mods,
            info: HashMap::new(),
            instance: instance.clone(),
            sender,

            mods_dir,
            resourcepacks_dir,
            shaderpacks_dir,
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

        let query_type = QueryType::from_modrinth_str(&project_info.project_type).ok_or(
            ModError::UnknownProjectType(project_info.project_type.clone()),
        )?;

        info!("Getting project info (id: {id})");

        if let QueryType::Mods | QueryType::ModPacks = query_type {
            if !self.has_compatible_loader(&project_info) {
                if let Some(loader) = &self.loader {
                    pt!("Mod {} doesn't support {loader}", project_info.title);
                } else {
                    err!("Mod {} doesn't support unknown loader!", project_info.title);
                }
                return Ok(());
            }
        }

        print_downloading_message(&project_info, dependent);

        let download_version = self
            .get_download_version(id, project_info.title.clone(), query_type)
            .await?;

        let mut dependency_list = HashSet::new();

        if QueryType::ModPacks != query_type {
            pt!("Getting dependencies");
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
        }

        if !self.index.mods.contains_key(id) {
            let not_allowed = self.download_file(&download_version, query_type).await?;
            // A modrinth modpack shouldn't download curseforge mods,
            // and curseforge mods can't be accidentally downloaded from modrinth.
            debug_assert!(not_allowed.is_empty());
            drop(not_allowed);

            self.add_mod_to_index(
                &project_info,
                &download_version,
                dependency_list,
                dependent,
                manually_installed,
                query_type,
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

        // Handling the same mod across multiple store backends
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

    async fn get_download_version(
        &self,
        id: &str,
        title: String,
        project_type: QueryType,
    ) -> Result<ModVersion, ModError> {
        pt!("Getting download info");
        let download_info = ModVersion::download(id).await?;

        let mut download_versions: Vec<ModVersion> = download_info
            .iter()
            .filter(|v| v.game_versions.contains(&self.version))
            .filter(|v| {
                if let (Some(loader), QueryType::Mods | QueryType::ModPacks) =
                    (&self.loader, project_type)
                {
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

    fn get_dir(&self, project_type: QueryType) -> Option<&Path> {
        match project_type {
            QueryType::Mods => Some(&self.mods_dir),
            QueryType::ResourcePacks => Some(&self.resourcepacks_dir),
            QueryType::Shaders => Some(&self.shaderpacks_dir),
            QueryType::ModPacks => None,
        }
    }

    async fn download_file(
        &self,
        download_version: &ModVersion,
        project_type: QueryType,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
        if let Some(primary_file) = download_version.files.iter().find(|file| file.primary) {
            if let QueryType::ModPacks = project_type {
                let bytes = file_utils::download_file_to_bytes(&primary_file.url, true).await?;
                return Ok(
                    install_modpack(bytes, self.instance.clone(), self.sender.as_ref())
                        .await
                        .map_err(Box::new)?,
                );
            } else {
                let file_path = self
                    .get_dir(project_type)
                    .unwrap()
                    .join(&primary_file.filename);
                file_utils::download_file_to_path(&primary_file.url, true, &file_path).await?;
            }
        } else {
            pt!("Didn't find primary file, checking secondary files...");
            for file in &download_version.files {
                if let QueryType::ModPacks = project_type {
                    let bytes = file_utils::download_file_to_bytes(&file.url, true).await?;
                    return Ok(
                        install_modpack(bytes, self.instance.clone(), self.sender.as_ref())
                            .await
                            .map_err(Box::new)?,
                    );
                } else {
                    let file_path = self.get_dir(project_type).unwrap().join(&file.filename);
                    file_utils::download_file_to_path(&file.url, true, &file_path).await?;
                }
            }
        }
        Ok(HashSet::new())
    }

    fn add_mod_to_index(
        &mut self,
        project_info: &ProjectInfo,
        download_version: &ModVersion,
        dependency_list: HashSet<String>,
        dependent: Option<&str>,
        manually_installed: bool,
        project_type: QueryType,
    ) {
        let config = ModConfig {
            name: project_info.title.clone(),
            description: project_info.description.clone(),
            icon_url: project_info.icon_url.clone(),
            project_id: project_info.id.clone(),
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
        };

        if let QueryType::Mods = project_type {
            self.index.mods.insert(project_info.id.clone(), config);
        }
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
    let instance_dir = instance.get_instance_path();
    let config_json = InstanceConfigJson::read_from_path(&instance_dir).await?;

    Ok(match config_json.mod_type.as_str() {
        "Fabric" => Some("fabric"),
        "Forge" => Some("forge"),
        "Quilt" => Some("quilt"),
        "NeoForge" => Some("neoforge"),
        "LiteLoader" => Some("liteloader"),
        "Rift" => Some("rift"),
        loader => {
            if loader != "Vanilla" {
                err!("Unknown loader {loader}");
            }
            None
        } // TODO: Add more loaders
    }
    .map(str::to_owned))
}
