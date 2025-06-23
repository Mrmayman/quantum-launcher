use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::Sender,
};

use ql_core::{
    err, file_utils, info, json::VersionDetails, pt, GenericProgress, InstanceSelection, ModId,
};

use crate::store::{
    curseforge::{get_query_type, ModQuery},
    get_loader, get_mods_resourcepacks_shaderpacks_dir, install_modpack, CurseforgeNotAllowed,
    ModConfig, ModError, ModFile, ModIndex, QueryType, SOURCE_ID_CURSEFORGE,
};

use super::Mod;

pub struct ModDownloader<'a> {
    version: String,
    loader: Option<String>,
    pub index: ModIndex,
    mods_dir: PathBuf,
    resourcepacks_dir: PathBuf,
    shaderpacks_dir: PathBuf,
    pub query_cache: HashMap<String, Mod>,
    instance: InstanceSelection,
    pub sender: Option<&'a Sender<GenericProgress>>,
    pub not_allowed: HashSet<CurseforgeNotAllowed>,
}

impl<'a> ModDownloader<'a> {
    pub async fn new(
        instance: InstanceSelection,
        sender: Option<&'a Sender<GenericProgress>>,
    ) -> Result<Self, ModError> {
        let version_json = VersionDetails::load(&instance).await?;
        let (mods_dir, resourcepacks_dir, shaderpacks_dir) =
            get_mods_resourcepacks_shaderpacks_dir(&instance, &version_json).await?;

        Ok(Self {
            version: version_json.id,
            loader: get_loader(&instance)
                .await?
                .map(|n| n.to_curseforge().to_owned()),
            index: ModIndex::get(&instance).await?,
            mods_dir,
            resourcepacks_dir,
            shaderpacks_dir,
            query_cache: HashMap::new(),
            instance,
            sender,
            not_allowed: HashSet::new(),
        })
    }

    pub async fn download(&mut self, id: &str, dependent: Option<&str>) -> Result<(), ModError> {
        // Mod already installed.
        if let Some(config) = self.index.mods.get_mut(id) {
            // Is this mod a dependency of something else?
            if let Some(dependent) = dependent {
                config.dependents.insert(format!("CF:{dependent}"));
            } else {
                config.manually_installed = true;
            }
            return Ok(());
        }

        info!("Installing mod {id}");
        let response = self.get_query(id).await?;
        pt!("Name: {}", response.name);

        if let Some(config) = self
            .index
            .mods
            .values_mut()
            .find(|n| n.name == response.name)
        {
            pt!("Already installed from modrinth? Skipping...");
            // Is this mod a dependency of something else?
            if let Some(dependent) = dependent {
                config.dependents.insert(format!("CF:{dependent}"));
            } else {
                config.manually_installed = true;
            }
            return Ok(());
        }

        let query_type = get_query_type(response.classId).await?;

        let (file_query, file_id) = response
            .get_file(
                response.name.clone(),
                id,
                &self.version,
                self.loader.as_deref(),
                query_type,
            )
            .await?;
        let Some(url) = file_query.data.downloadUrl.clone() else {
            self.not_allowed.insert(CurseforgeNotAllowed {
                name: response.name.clone(),
                slug: response.slug.clone(),
                filename: file_query.data.fileName.clone(),
                project_type: query_type.to_curseforge_str().to_owned(),
                file_id: file_id as usize,
            });
            return Ok(());
        };

        let dir = match query_type {
            QueryType::Mods => &self.mods_dir,
            QueryType::ResourcePacks => &self.resourcepacks_dir,
            QueryType::Shaders => &self.shaderpacks_dir,
            QueryType::ModPacks => {
                let bytes = file_utils::download_file_to_bytes(&url, true).await?;
                if let Some(not_allowed_new) =
                    install_modpack(bytes, self.instance.clone(), self.sender)
                        .await
                        .map_err(Box::new)?
                {
                    self.not_allowed.extend(not_allowed_new);
                } else {
                    err!("Invalid modpack downloaded from curseforge! Corrupted?");
                }
                return Ok(());
            }
        };

        let file_dir = dir.join(&file_query.data.fileName);
        file_utils::download_file_to_path(&url, true, &file_dir).await?;

        let id_str = response.id.to_string();
        let id_mod = ModId::Curseforge(id_str.clone());

        for dependency in &file_query.data.dependencies {
            let dep_id = dependency.modId.to_string();
            pt!("Installing dependency {dep_id}");
            Box::pin(self.download(&dep_id, Some(id))).await?;
        }

        self.add_to_index(dependent, &response, query_type, file_query, url, &id_mod);

        pt!("Finished installing {query_type}: {}", response.name);

        Ok(())
    }

    fn add_to_index(
        &mut self,
        dependent: Option<&str>,
        response: &Mod,
        query_type: QueryType,
        file_query: super::CurseforgeFileQuery,
        url: String,
        id_mod: &ModId,
    ) {
        if let QueryType::Mods = query_type {
            let id_index_str = id_mod.get_index_str();
            self.index.mods.insert(
                id_index_str.clone(),
                ModConfig {
                    name: response.name.clone(),
                    manually_installed: dependent.is_none(),
                    installed_version: file_query.data.displayName.clone(),
                    version_release_time: file_query.data.fileDate.clone(),
                    enabled: true,
                    description: response.summary.clone(),
                    icon_url: response.logo.clone().map(|n| n.url),
                    project_source: SOURCE_ID_CURSEFORGE.to_owned(),
                    project_id: id_index_str.clone(),
                    files: vec![ModFile {
                        url,
                        filename: file_query.data.fileName,
                        primary: true,
                    }],
                    supported_versions: file_query
                        .data
                        .gameVersions
                        .iter()
                        .filter(|n| n.contains('.'))
                        .cloned()
                        .collect(),
                    dependencies: file_query
                        .data
                        .dependencies
                        .into_iter()
                        .map(|n| format!("CF:{}", n.modId))
                        .collect(),
                    dependents: if let Some(dependent) = dependent {
                        let mut set = HashSet::new();
                        set.insert(format!("CF:{dependent}"));
                        set
                    } else {
                        HashSet::new()
                    },
                },
            );
        }
    }

    async fn get_query(&mut self, id: &str) -> Result<Mod, ModError> {
        Ok(if let Some(r) = self.query_cache.get(id) {
            r.clone()
        } else {
            let query = ModQuery::load(id).await?;
            self.query_cache.insert(id.to_owned(), query.data.clone());
            query.data
        })
    }
}
