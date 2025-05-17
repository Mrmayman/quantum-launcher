use std::{
    io::{Cursor, Read},
    path::Path,
};

use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, InstanceSelection, IntoIoError, IntoJsonError,
};

mod curseforge;
mod error;
mod modrinth;

pub use error::PackError;

pub async fn install_modpack(file: Vec<u8>, instance: InstanceSelection) -> Result<(), PackError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(file))?;

    info!("Installing modpack");

    let index_json_modrinth: Option<modrinth::PackIndex> =
        read_json_from_zip(&mut zip, "modrinth.index.json")?;
    let index_json_curseforge: Option<curseforge::PackIndex> =
        read_json_from_zip(&mut zip, "manifest.json")?;

    let overrides = index_json_curseforge
        .as_ref()
        .map(|n| n.overrides.clone())
        .unwrap_or("overrides".to_owned());

    let mc_dir = instance.get_dot_minecraft_path();
    let config = InstanceConfigJson::read(&instance).await?;
    let json = VersionDetails::load(&instance).await?;

    if let Some(index) = index_json_modrinth {
        install_modrinth(&instance, &mc_dir, &config, &json, &index).await?;
    }
    if let Some(index) = index_json_curseforge {
        install_curseforge(&instance, &config, &json, &index).await?;
    }

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_owned();

        if name == "modrinth.index.json" || name == "manifest.json" || name == "modlist.html" {
            continue;
        }

        if let Some(name) = name
            .strip_prefix(&format!("{overrides}/"))
            .or(name.strip_prefix(&format!("{overrides}\\")))
        {
            let path = mc_dir.join(name);
            let parent = if file.is_dir() {
                &path
            } else {
                let Some(parent) = path.parent() else {
                    continue;
                };
                parent
            };
            tokio::fs::create_dir_all(parent).await.path(parent)?;

            if file.is_file() {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|n| PackError::ZipIoError(n, name.to_owned()))?;

                tokio::fs::write(&path, &buf).await.path(&path)?;
            }
        } else {
            err!("Unrecognised file: {name}");
        }
    }

    Ok(())
}

async fn install_modrinth(
    instance: &InstanceSelection,
    mc_dir: &Path,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &modrinth::PackIndex,
) -> Result<(), PackError> {
    if let Some(version) = index.dependencies.get("minecraft") {
        if json.id != *version {
            return Err(PackError::GameVersion {
                expect: version.clone(),
                got: json.id.clone(),
            });
        }
    }

    pt!("Modrinth Modpack: {}", index.name);
    let loader = match config.mod_type.as_str() {
        "Forge" => "forge",
        "Fabric" => "fabric-loader",
        "Quilt" => "quilt-loader",
        "NeoForge" => "neoforge",
        _ => {
            return Err(expect_got_modrinth(index, config));
        }
    };
    if !index.dependencies.contains_key(loader) {
        return Err(expect_got_modrinth(index, config));
    }

    for file in &index.files {
        let required_field = match instance {
            InstanceSelection::Instance(_) => &file.env.client,
            InstanceSelection::Server(_) => &file.env.server,
        };
        if required_field != "required" {
            pt!("Skipping {} (optional)", file.path);
            continue;
        }

        let Some(download) = file.downloads.first() else {
            pt!("No downloads found for {}, skipping...", file.path);
            continue;
        };

        let bytes = file_utils::download_file_to_bytes(download, true).await?;
        let bytes_path = mc_dir.join(&file.path);
        tokio::fs::write(&bytes_path, &bytes)
            .await
            .path(bytes_path)?;
    }

    Ok(())
}

async fn install_curseforge(
    instance: &InstanceSelection,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &curseforge::PackIndex,
) -> Result<(), PackError> {
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
        .filter_map(|n| n.get("id"))
        .any(|n| n.starts_with(loader))
    {
        return Err(expect_got_curseforge(index, config));
    }

    let mut not_allowed = Vec::new();
    for file in &index.files {
        file.download(&mut not_allowed, instance, json).await?;
    }

    Ok(())
}

fn expect_got_curseforge(index: &curseforge::PackIndex, config: &InstanceConfigJson) -> PackError {
    PackError::Loader {
        expect: index
            .minecraft
            .modLoaders
            .iter()
            .filter_map(|l| l.get("id"))
            .map(|l| l.split('-').next().unwrap_or(l))
            .collect::<Vec<&str>>()
            .join(", "),
        got: config.mod_type.clone(),
    }
}

fn read_json_from_zip<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    name: &str,
) -> Result<Option<T>, PackError> {
    Ok(if let Ok(mut index_file) = zip.by_name(name) {
        let mut buf = Vec::new();
        index_file
            .read_to_end(&mut buf)
            .map_err(|n| PackError::ZipIoError(n, name.to_owned()))?;

        Some(
            serde_json::from_slice(&buf).json(
                String::from_utf8(buf.clone())
                    .ok()
                    .unwrap_or_else(|| String::from_utf8_lossy(&buf).to_string()),
            )?,
        )
    } else {
        None
    })
}

fn expect_got_modrinth(index_json: &modrinth::PackIndex, config: &InstanceConfigJson) -> PackError {
    match index_json
        .dependencies
        .iter()
        .filter_map(|(k, _)| (k != "minecraft").then_some(k.clone()))
        .map(|loader| {
            loader
                .strip_suffix("-loader")
                .map(|n| n.to_owned())
                .unwrap_or(loader)
        })
        .next()
    {
        Some(expect) => PackError::Loader {
            expect,
            got: config.mod_type.clone(),
        },
        None => PackError::NoLoadersSpecified,
    }
}
