use std::{collections::HashSet, sync::mpsc::Sender};

use ql_core::{
    do_jobs, file_utils,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError,
};
use serde::Deserialize;
use tokio::sync::Mutex;

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
        not_allowed: &Mutex<HashSet<CurseforgeNotAllowed>>,
        instance: &InstanceSelection,
        json: &VersionDetails,
        sender: Option<&Sender<GenericProgress>>,
        (i, len): (&Mutex<usize>, usize),
    ) -> Result<(), PackError> {
        if !self.required {
            return Ok(());
        }

        let project_id = self.projectID.to_string();

        let mod_info = ModQuery::load(&project_id).await?;
        let query = CurseforgeFileQuery::load(&project_id, self.fileID as i32).await?;
        let query_type = get_query_type(mod_info.data.classId).await?;

        let Some(url) = query.data.downloadUrl.clone() else {
            not_allowed.lock().await.insert(CurseforgeNotAllowed {
                name: mod_info.data.name,
                slug: mod_info.data.slug,
                file_id: self.fileID,
                project_type: query_type.to_curseforge_str().to_owned(),
                filename: query.data.fileName,
            });
            return Ok(());
        };

        let (dir_mods, dir_res_packs, dir_shader) =
            get_mods_resourcepacks_shaderpacks_dir(instance, json).await?;
        let dir = match query_type {
            QueryType::Mods => dir_mods,
            QueryType::ResourcePacks => dir_res_packs,
            QueryType::Shaders => dir_shader,
            QueryType::ModPacks => return Err(PackError::ModpackInModpack),
        };

        let path = dir.join(query.data.fileName);
        if path.is_file() {
            let metadata = tokio::fs::metadata(&path).await.path(&path)?;
            let got_len = metadata.len();
            if query.data.fileLength == got_len {
                pt!("Already installed {}, skipping", mod_info.data.name);
                return Ok(());
            }
        }

        file_utils::download_file_to_path(&url, true, &path).await?;

        if let Some(sender) = sender {
            let mut i = i.lock().await;
            _ = sender.send(GenericProgress {
                done: *i,
                total: len,
                message: Some(format!(
                    "Modpack: Installed mod (curseforge) ({i}/{len}):\n{}",
                    mod_info.data.name,
                    i = *i + 1,
                )),
                has_finished: false,
            });
            pt!(
                "Installed mod (curseforge) ({i}/{len}): {}",
                mod_info.data.name,
                i = *i + 1,
            );
            *i += 1;
        }

        Ok(())
    }
}

pub async fn install(
    instance: &InstanceSelection,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &PackIndex,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
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

    let not_allowed = Mutex::new(HashSet::new());
    let len = index.files.len();

    let i = Mutex::new(0);

    let jobs: Result<Vec<()>, PackError> = do_jobs(
        index
            .files
            .iter()
            .map(|file| file.download(&not_allowed, instance, json, sender, (&i, len))),
    )
    .await;
    jobs?;

    let not_allowed = not_allowed.lock().await;
    Ok(not_allowed.clone())
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
