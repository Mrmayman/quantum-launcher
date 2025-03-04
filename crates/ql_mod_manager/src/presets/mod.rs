use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoStringError, SelectedMod,
    LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use zip::ZipWriter;

use crate::{
    mod_manager::{ModConfig, ModDownloader, ModError, ModIndex},
    rate_limiter::MOD_DOWNLOAD_LOCK,
};

/// A "Mod Preset"
///
/// # What are mod presets?
/// Mod presets are essentially "bundles" or "packs"
/// of mods. Think modpacks, but with a different, probably
/// better format.
///
/// They include
/// - Installed mods (both from store and from outside)
/// - Mod configuration
///
/// # How to use this?
/// See the [`PresetJson::generate`], [`PresetJson::load`],
/// and [`PresetJson::download_entries_w`] functions.
///
/// # Format
/// Mod presets consist of a `.qmp` file
/// (it's actually a zip, can be any extension you want).
///
/// Inside this zip file, there will be:
/// - An `index.json` file, essentially a `serde::Serialize`d
///   version of [`PresetJson`] (the main struct through which
///   this API is used).
/// - `.jar` files in the root of the zip (at the top level),
///   for any local, sideloaded mods from outside the mod store.
///   **Note: mods installed through the mod store shouldn't be saved
///   here, but rather their details should be entered in the `index.json`
/// - All configuration files in a `config/` folder. This will be extracted
///   to the `.minecraft/config/` folder
#[derive(Serialize, Deserialize)]
pub struct PresetJson {
    pub launcher_version: String,
    pub minecraft_version: String,
    pub instance_type: String,
    pub entries_modrinth: HashMap<String, ModConfig>,
    pub entries_local: Vec<String>,
}

impl PresetJson {
    pub async fn generate(
        instance_name: InstanceSelection,
        selected_mods: HashSet<SelectedMod>,
    ) -> Result<Vec<u8>, ModError> {
        let dot_minecraft = file_utils::get_dot_minecraft_dir(&instance_name).await?;
        let mods_dir = dot_minecraft.join("mods");
        let config_dir = dot_minecraft.join("config");

        let minecraft_version = get_minecraft_version(&instance_name).await?;
        let instance_type = get_instance_type(&instance_name).await?;

        let index = ModIndex::get(&instance_name).await?;

        let mut entries_modrinth = HashMap::new();
        let mut entries_local: Vec<(String, Vec<u8>)> = Vec::new();

        for entry in selected_mods {
            match entry {
                SelectedMod::Downloaded { id, .. } => {
                    add_mod_to_entries_modrinth(&mut entries_modrinth, &index, &id);
                }
                SelectedMod::Local { file_name } => {
                    if is_already_covered(&index, &file_name) {
                        continue;
                    }

                    let entry = mods_dir.join(&file_name);
                    let mod_bytes = tokio::fs::read(&entry).await.path(&entry)?;
                    entries_local.push((file_name.clone(), mod_bytes));
                }
            }
        }

        let this = Self {
            instance_type,
            launcher_version: LAUNCHER_VERSION_NAME.to_owned(),
            minecraft_version,
            entries_modrinth,
            entries_local: entries_local.iter().map(|(n, _)| n).cloned().collect(),
        };

        let file: Vec<u8> = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(file));

        for (name, bytes) in entries_local {
            zip.start_file(&name, zip::write::FileOptions::<()>::default())?;
            zip.write_all(&bytes)
                .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
        }

        if config_dir.is_dir() {
            add_dir_to_zip_recursive(&config_dir, &mut zip, PathBuf::from("config")).await?;
        }

        zip.start_file("index.json", zip::write::FileOptions::<()>::default())?;
        let this_str = serde_json::to_string(&this)?;
        let this_str = this_str.as_bytes();
        zip.write_all(this_str)
            .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;

        let file = zip.finish()?.get_ref().clone();
        info!("Built mod preset! Size: {} bytes", file.len());

        Ok(file)
    }

    /// Loads a `.qmp` file as a "Mod Preset".
    ///
    /// See the module documentation for what a preset is.
    ///
    /// # Arguments
    /// - `instance_name: InstanceSelection`:
    ///   The instance to which the preset will be installed.
    /// - `zip: Vec<u8>`:
    ///   The `.qmp` file in binary form. Must be read from
    ///   disk earlier.
    ///
    /// Returns a Vec<String> of modrinth mod id's to be installed
    /// to "complete" the installation. You pass this to
    /// [`PresetJson::download_entries_w`]
    ///
    /// # Errors
    /// - The provided `zip` is not a valid `.zip` file.
    /// - `index.json` in the zip file isn't valid JSON
    /// - User lacks permission to access `QuantumLauncher/` folder
    /// ---
    /// - `details.json` file couldn't be loaded
    /// - `details.json` couldn't be parsed into valid JSON
    /// ---
    /// - `config.json` file couldn't be loaded
    /// - `config.json` couldn't be parsed into valid JSON
    /// ---
    /// - instance directory is outside the launcher directory (escape attack)
    /// - config dir (~/.config on linux or AppData/Roaming on windows) is not found
    /// - the launcher directory could not be created (permissions issue)
    /// ---
    /// - And many other stuff I probably forgot
    pub async fn load(
        instance_name: InstanceSelection,
        zip: Vec<u8>,
    ) -> Result<Vec<String>, ModError> {
        info!("Importing mod preset");

        let main_dir = file_utils::get_dot_minecraft_dir(&instance_name).await?;
        let mods_dir = main_dir.join("mods");

        let mut zip = zip::ZipArchive::new(Cursor::new(zip)).map_err(ModError::Zip)?;

        let mut entries_modrinth = HashMap::new();

        let version_json = VersionDetails::load(&instance_name).await?;
        let mut sideloads = Vec::new();
        let mut should_sideload = true;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(ModError::Zip)?;
            let name = file.name().to_owned();

            if name == "index.json" {
                pt!("Mod index");
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
                let this: Self = serde_json::from_slice(&buf)?;

                // Only sideload mods if the version is the same
                let instance_type = get_instance_type(&instance_name).await?;

                should_sideload = this.minecraft_version == version_json.id
                    && this.instance_type == instance_type;

                // If sideloaded mods are of an incompatible version, remove them
                if !should_sideload {
                    for file in &sideloads {
                        let path = mods_dir.join(file);
                        tokio::fs::remove_file(&path).await.path(&path)?;
                    }
                }
                entries_modrinth = this.entries_modrinth;
            } else if name.starts_with("config/") || name.starts_with("config\\") {
                pt!("Config file: {name}");
                let path = main_dir.join(name.replace('\\', "/"));

                if file.is_dir() {
                    tokio::fs::create_dir_all(&path).await.path(&path)?;
                } else {
                    let parent = path.parent().unwrap();
                    tokio::fs::create_dir_all(parent).await.path(parent)?;

                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf)
                        .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
                    tokio::fs::write(&path, &buf).await.path(&path)?;
                }
            } else if name.contains('/') || name.contains('\\') {
                info!("Feature not implemented: {name}");
            } else {
                if !should_sideload {
                    continue;
                }
                pt!("Local file: {name}");
                let path = mods_dir.join(&name);
                sideloads.push(name.clone());
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|n| ModError::ZipIoError(n, name))?;
                tokio::fs::write(&path, &buf).await.path(&path)?;
            }
        }

        let mods = entries_modrinth
            .into_values()
            .filter_map(|n| n.manually_installed.then_some(n.project_id))
            .collect();

        Ok(mods)
    }

    pub async fn download_entries_w(
        ids: Vec<String>,
        instance_name: InstanceSelection,
        sender: Sender<GenericProgress>,
    ) -> Result<(), String> {
        let len = ids.len();

        let _guard = if let Ok(g) = MOD_DOWNLOAD_LOCK.try_lock() {
            g
        } else {
            info!("Another mod is already being installed... Waiting...");
            MOD_DOWNLOAD_LOCK.lock().await
        };

        let mut downloader = ModDownloader::new(&instance_name).await.strerr()?;

        for (i, id) in ids.into_iter().enumerate() {
            _ = sender.send(GenericProgress {
                done: i,
                total: len,
                message: None,
                has_finished: false,
            });

            let result = downloader.download_project(&id, None, true).await;
            if let Err(ModError::NoCompatibleVersionFound) = result {
                err!("Mod {id} is not compatible with this version. Skipping...");
                continue;
            }
            result.strerr()?;

            if let Some(config) = downloader.index.mods.get_mut(&id) {
                config.manually_installed = true;
            }
        }

        downloader.index.save().await.strerr()?;

        _ = sender.send(GenericProgress::finished());
        Ok(())
    }
}

async fn get_instance_type(instance_name: &InstanceSelection) -> Result<String, ModError> {
    let config_path = file_utils::get_instance_dir(instance_name).await?;
    let config = InstanceConfigJson::read_from_path(&config_path).await?;
    Ok(config.mod_type)
}

fn add_mod_to_entries_modrinth(
    entries_modrinth: &mut HashMap<String, ModConfig>,
    index: &ModIndex,
    id: &str,
) {
    let Some(config) = index.mods.get(id) else {
        err!("Could not find id {id} in index!");
        return;
    };

    entries_modrinth.insert(id.to_owned(), config.clone());

    for dep in &config.dependencies {
        add_mod_to_entries_modrinth(entries_modrinth, index, dep);
    }
}

async fn get_minecraft_version(instance_name: &InstanceSelection) -> Result<String, ModError> {
    let version_json = VersionDetails::load(instance_name).await?;
    let minecraft_version = version_json.id.clone();
    Ok(minecraft_version)
}

async fn add_dir_to_zip_recursive(
    path: &Path,
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    accumulation: PathBuf,
) -> Result<(), ModError> {
    let mut dir = tokio::fs::read_dir(path).await.path(path)?;

    // # Explanation
    // For example, if the dir structure is:
    //
    // config
    // |- file1.txt
    // |- file2.txt
    // |- dir1
    // | |- file3.txt
    // | |- file4.txt
    //
    // Assume accumulation is "config" for example...

    while let Some(entry) = dir.next_entry().await.path(path)? {
        let path = entry.path();
        let accumulation = accumulation.join(path.file_name().unwrap().to_str().unwrap());

        if path.is_dir() {
            zip.add_directory(
                format!("{}/", accumulation.to_str().unwrap()),
                zip::write::FileOptions::<()>::default(),
            )
            .map_err(ModError::Zip)?;

            // ... accumulation = "config/dir1"
            // Then this call will have "config/dir1" as starting value.
            Box::pin(add_dir_to_zip_recursive(&path, zip, accumulation.clone())).await?;
        } else {
            // ... accumulation = "config/file1.txt"
            let bytes = tokio::fs::read(&path).await.path(path.clone())?;
            zip.start_file(
                accumulation.to_str().unwrap(),
                zip::write::FileOptions::<()>::default(),
            )?;
            zip.write_all(&bytes)
                .map_err(|n| ModError::ZipIoError(n, accumulation.to_str().unwrap().to_owned()))?;
        }
    }

    Ok(())
}

fn is_already_covered(index: &ModIndex, mod_name: &String) -> bool {
    for config in index.mods.values() {
        if config.files.iter().any(|n| n.filename == *mod_name) {
            return true;
        }
    }
    false
}
