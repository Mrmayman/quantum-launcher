use std::sync::mpsc::Sender;

use ql_core::{
    file_utils,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError,
};
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
    pub modLoaders: Vec<PackLoader>,
    // No one asked for your recommendation bro:
    // pub recommendedRam: usize
}

#[derive(Deserialize)]
pub struct PackLoader {
    pub id: String,
    // pub primary: bool,
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
        sender: Option<&Sender<GenericProgress>>,
        (i, len): (usize, usize),
    ) -> Result<(), PackError> {
        if !self.required {
            return Ok(());
        }

        let project_id = self.projectID.to_string();

        let mod_info = ModQuery::load(&project_id).await?;
        let query = CurseforgeFileQuery::load(&project_id, self.fileID as i32).await?;

        if let Some(sender) = sender {
            _ = sender.send(GenericProgress {
                done: i,
                total: len,
                message: Some(format!(
                    "Modpack: Installing mod (curseforge): {} ({i}/{len})",
                    mod_info.data.name,
                    i = i + 1
                )),
                has_finished: false,
            });
        }

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

pub async fn install(
    instance: &InstanceSelection,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &PackIndex,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<Vec<CurseforgeNotAllowed>, PackError> {
    if json.id != index.minecraft.version {
        return Err(PackError::GameVersion {
            expect: index.minecraft.version.clone(),
            got: json.id.clone(),
        });
    }

    pt!("CurseForge Modpack: {}", index.name);

    let loader = match config.mod_type.as_str() {
        "Forge" => "forge",
        "Fabric" => "fabric",
        "Quilt" => "quilt",
        "NeoForge" => "neoforge",
        _ => {
            return Err(expect_got_curseforge(index, config));
        }
    };

    if !index
        .minecraft
        .modLoaders
        .iter()
        .any(|n| n.id.starts_with(loader))
    {
        return Err(expect_got_curseforge(index, config));
    }

    let mut not_allowed = Vec::new();
    let len = index.files.len();
    for (i, file) in index.files.iter().enumerate() {
        file.download(&mut not_allowed, instance, json, sender, (i, len))
            .await?;
    }

    Ok(not_allowed)
}

fn expect_got_curseforge(index: &PackIndex, config: &InstanceConfigJson) -> PackError {
    PackError::Loader {
        expect: index
            .minecraft
            .modLoaders
            .iter()
            .map(|l| l.id.split('-').next().unwrap_or(&l.id))
            .collect::<Vec<&str>>()
            .join(", "),
        got: config.mod_type.clone(),
    }
}
