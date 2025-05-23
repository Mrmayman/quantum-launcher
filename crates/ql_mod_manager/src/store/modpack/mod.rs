use std::{
    collections::HashSet,
    io::{Cursor, Read},
    sync::mpsc::Sender,
};

use ql_core::{
    err, info,
    json::{InstanceConfigJson, VersionDetails},
    GenericProgress, InstanceSelection, IntoIoError, IntoJsonError,
};

mod curseforge;
mod error;
mod modrinth;

pub use error::PackError;

use super::CurseforgeNotAllowed;

/// Installs a modpack file.
///
/// Not to be confused with [`PresetJson`]
/// (`.qmp` mod presets). Those are QuantumLauncher-only,
/// but these ones are found across the internet.
///
/// This function supports both Curseforge and Modrinth modpacks,
/// it doesn't matter which one you put in.
///
/// # Arguments
/// - `file: Vec<u8>`: The bytes of the modpack file.
/// - `instance: InstanceSelection`: The selected instance you want to download this pack to.
/// - `sender: Option<&Sender<GenericProgress>>`: Supply a [`std::sync::mpsc::Sender`] if you want
///   to see the progress of installation. Leave `None` if otherwise.
///
/// # Returns
/// - `Ok(HashSet<CurseforgeNotAllowed)` - The list of mods that
///   Curseforge blocked the launcher from automatically downloading. The user must
///   manually download these from the browser and import them.
/// - `Err` - Any error that occured.
pub async fn install_modpack(
    file: Vec<u8>,
    instance: InstanceSelection,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
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
        modrinth::install(&instance, &mc_dir, &config, &json, &index, sender).await?;
    }
    let not_allowed = if let Some(index) = index_json_curseforge {
        curseforge::install(&instance, &config, &json, &index, sender).await?
    } else {
        HashSet::new()
    };

    let len = zip.len();
    for i in 0..len {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_owned();

        if name == "modrinth.index.json" || name == "manifest.json" || name == "modlist.html" {
            continue;
        }

        if let Some(sender) = sender {
            _ = sender.send(GenericProgress {
                done: i,
                total: len,
                message: Some(format!(
                    "Modpack: Creating overrides: {name} ({i}/{len})",
                    i = i + 1
                )),
                has_finished: false,
            });
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

    Ok(not_allowed)
}

fn read_json_from_zip<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    name: &str,
) -> Result<Option<T>, PackError> {
    Ok(if let Ok(mut index_file) = zip.by_name(name) {
        let buf = std::io::read_to_string(&mut index_file)
            .map_err(|n| PackError::ZipIoError(n, name.to_owned()))?;

        Some(serde_json::from_str(&buf).json(buf)?)
    } else {
        None
    })
}
