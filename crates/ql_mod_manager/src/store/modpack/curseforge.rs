use std::collections::HashMap;

use ql_core::{file_utils, json::VersionDetails, InstanceSelection, IntoIoError};
use serde::Deserialize;

use crate::store::{
    curseforge::{get_query_type, CurseforgeFileQuery, ModQuery},
    get_mods_resourcepacks_shaderpacks_dir, CurseforgeNotAllowed, QueryType,
};

use super::PackError;

#[derive(Deserialize)]
pub struct PackIndex {
    pub minecraft: PackMinecraft,
    pub name: String,
    pub files: Vec<PackFile>,
    pub overrides: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct PackMinecraft {
    pub version: String,
    pub modLoaders: Vec<HashMap<String, String>>,
    // No one asked for your recommendation bro:
    // pub recommendedRam: usize
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct PackFile {
    pub projectID: usize,
    pub fileID: usize,
    pub required: bool,
}

impl PackFile {
    pub async fn download(
        &self,
        not_allowed: &mut Vec<CurseforgeNotAllowed>,
        instance: &InstanceSelection,
        json: &VersionDetails,
    ) -> Result<(), PackError> {
        if !self.required {
            return Ok(());
        }

        let project_id = self.projectID.to_string();

        let mod_info = ModQuery::load(&project_id).await?;
        let query = CurseforgeFileQuery::load(&project_id, self.fileID as i32).await?;

        let Some(url) = query.data.downloadUrl.clone() else {
            not_allowed.push(CurseforgeNotAllowed {
                name: mod_info.data.name,
                slug: mod_info.data.slug,
                id: self.fileID,
            });
            return Ok(());
        };

        let query_type = get_query_type(mod_info.data.classId).await?;

        let (dir_mods, dir_res_packs, dir_shader) =
            get_mods_resourcepacks_shaderpacks_dir(instance, json).await?;
        let dir = match query_type {
            QueryType::Mods => dir_mods,
            QueryType::ResourcePacks => dir_res_packs,
            QueryType::Shaders => dir_shader,
        };

        let bytes = file_utils::download_file_to_bytes(&url, true).await?;
        let path = dir.join(query.data.fileName);
        tokio::fs::write(&path, &bytes).await.path(&path)?;
        Ok(())
    }
}
