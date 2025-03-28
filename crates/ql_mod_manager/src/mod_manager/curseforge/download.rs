use std::{collections::HashSet, path::Path};

use ql_core::{file_utils, info, pt, IntoIoError, ModId};

use crate::mod_manager::{
    curseforge::ModQuery, ModConfig, ModError, ModFile, ModIndex, SOURCE_ID_CURSEFORGE,
};

pub async fn download(
    id: &str,
    version: &str,
    loader: Option<&str>,
    index: &mut ModIndex,
    mods_dir: &Path,
    dependent: Option<&str>,
) -> Result<(), ModError> {
    // Mod already installed.
    if let Some(config) = index.mods.get_mut(id) {
        // Is this mod a dependency of something else?
        if let Some(dependent) = dependent {
            config.dependents.insert(format!("CF:{dependent}"));
        } else {
            config.manually_installed = true;
        }
        return Ok(());
    }

    info!("Installing mod {id}");
    let response = ModQuery::load(id).await?;
    pt!("name: {}", response.data.name);

    if let Some(config) = index
        .mods
        .values_mut()
        .find(|n| n.name == response.data.name)
    {
        pt!("Already installed from modrinth? Skipping...");
        // Is this mod a dependency of something else?
        if let Some(dependent) = dependent {
            config.dependents.insert(dependent.to_owned());
        } else {
            config.manually_installed = true;
        }
        return Ok(());
    }

    let file_query = response.get_file(id, version, loader).await?;
    let Some(url) = file_query.data.downloadUrl.clone() else {
        return Err(ModError::CurseforgeModNotAllowedForDownload(
            response.data.name.clone(),
            response.data.slug.clone(),
        ));
    };

    let bytes = file_utils::download_file_to_bytes(&url, true).await?;
    let file_dir = mods_dir.join(&file_query.data.fileName);
    tokio::fs::write(&file_dir, &bytes).await.path(&file_dir)?;

    let id_str = response.data.id.to_string();
    let id_mod = ModId::Curseforge(id_str.clone());

    println!("{:?}", file_query.data.dependencies);
    for dependency in &file_query.data.dependencies {
        let dep_id = dependency.modId.to_string();
        pt!("Installing dependency {dep_id}");
        Box::pin(download(
            &dep_id,
            version,
            loader,
            index,
            mods_dir,
            Some(id),
        ))
        .await?;
    }

    let id_index_str = id_mod.get_index_str();
    index.mods.insert(
        id_index_str.clone(),
        ModConfig {
            name: response.data.name.clone(),
            manually_installed: dependent.is_none(),
            installed_version: file_query.data.displayName.clone(),
            version_release_time: file_query.data.fileDate.clone(),
            enabled: true,
            description: response.data.summary.clone(),
            icon_url: response.data.logo.clone().map(|n| n.url),
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
                .map(|n| n.modId.to_string())
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

    pt!("Finished installing mod: {}", response.data.name);

    Ok(())
}
