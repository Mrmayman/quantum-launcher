use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use ql_core::{
    err, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, InstanceSelection, IntoIoError, IntoJsonError, ModId, SelectedMod, LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use zip::ZipWriter;

use crate::store::{ModConfig, ModError, ModIndex};

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
/// See the [`PresetJson::generate`] and [`PresetJson::load`],
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
    /// Generates a "Mod Preset" from the mods
    /// installed in the `instance`.
    ///
    /// This packages the contents of
    /// `.minecraft/mods` and `.minecraft/config`
    /// into a `.qmp` file (a specialized ZIP file).
    ///
    /// You have to manually provide which of the
    /// instance's mods you want through the `selected_mods`
    /// argument. You *can't* leave it empty, or nothing
    /// will generate.
    ///
    /// This returns a `Result` of `Vec<u8>`, containing
    /// the bytes of the final `.qmp` file that you can save
    /// anywhere you want.
    pub async fn generate(
        instance: InstanceSelection,
        selected_mods: HashSet<SelectedMod>,
    ) -> Result<Vec<u8>, ModError> {
        let dot_minecraft = instance.get_dot_minecraft_path();
        let mods_dir = dot_minecraft.join("mods");
        let config_dir = dot_minecraft.join("config");

        let minecraft_version = get_minecraft_version(&instance).await?;
        let instance_type = get_instance_type(&instance).await?;

        let index = ModIndex::get(&instance).await?;

        let mut entries_modrinth = HashMap::new();
        let mut entries_local: Vec<(String, Vec<u8>)> = Vec::new();

        for entry in selected_mods {
            match entry {
                SelectedMod::Downloaded { id, .. } => {
                    add_downloaded_mod_to_entries(&mut entries_modrinth, &index, &id);
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
        let this_str = serde_json::to_string(&this).json_to()?;
        let this_str = this_str.as_bytes();
        zip.write_all(this_str)
            .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;

        let file = zip.finish()?.get_ref().clone();
        info!("Built mod preset! Size: {} bytes", file.len());

        Ok(file)
    }

    /// Installs a `.qmp` file as a "Mod Preset".
    ///
    /// See the module documentation for what a preset is.
    ///
    /// # Arguments
    /// - `instance: InstanceSelection`:
    ///   The instance to which the preset will be installed.
    /// - `zip: Vec<u8>`:
    ///   The `.qmp` file in binary form. Must be read from
    ///   disk earlier.
    ///
    /// Returns a `Vec<String>` of modrinth mod id's to be installed
    /// to "complete" the installation. You pass this to
    /// [`crate::store::download_mods_bulk`]
    ///
    /// # Errors
    /// - The provided `zip` is not a valid `.zip` file.
    /// - `index.json` in the zip file isn't valid JSON
    /// - User lacks permission to access `QuantumLauncher/` folder
    /// - instance directory is outside the launcher directory (escape attack)
    /// ---
    /// `details.json` and `config.json` (in instance dir):
    /// - couldn't be loaded from disk
    /// - couldn't be parsed into valid JSON
    /// ---
    /// - And many other stuff I probably forgot
    pub async fn load(instance: InstanceSelection, zip: Vec<u8>) -> Result<Vec<ModId>, ModError> {
        info!("Importing mod preset");

        let main_dir = instance.get_dot_minecraft_path();
        let mods_dir = main_dir.join("mods");

        let mut zip = zip::ZipArchive::new(Cursor::new(zip)).map_err(ModError::Zip)?;

        let mut entries_downloaded = HashMap::new();

        let version_json = VersionDetails::load(&instance).await?;
        let mut sideloads = Vec::new();
        let mut should_sideload = true;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(ModError::Zip)?;
            let name = file.name().to_owned();

            if name == "index.json" {
                pt!("Mod index");
                let buf = std::io::read_to_string(&mut file)
                    .map_err(|n| ModError::ZipIoError(n, name.clone()))?;
                let this: Self = serde_json::from_str(&buf).json(buf)?;

                // Only sideload mods if the version is the same
                let instance_type = get_instance_type(&instance).await?;

                should_sideload = this.minecraft_version == version_json.id
                    && this.instance_type == instance_type;

                // If sideloaded mods are of an incompatible version, remove them
                if !should_sideload {
                    for file in &sideloads {
                        let path = mods_dir.join(file);
                        tokio::fs::remove_file(&path).await.path(&path)?;
                    }
                }
                entries_downloaded = this.entries_modrinth;
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

        let mods = entries_downloaded
            .into_iter()
            .filter_map(|(k, n)| n.manually_installed.then_some(ModId::from_index_str(&k)))
            .collect();

        Ok(mods)
    }
}

async fn get_instance_type(instance_name: &InstanceSelection) -> Result<String, ModError> {
    let config = InstanceConfigJson::read(instance_name).await?;
    Ok(config.mod_type)
}

fn add_downloaded_mod_to_entries(
    entries_modrinth: &mut HashMap<String, ModConfig>,
    index: &ModIndex,
    id: &ModId,
) {
    let id_str = id.get_index_str();
    let Some(config) = index.mods.get(&id_str) else {
        err!("Could not find id {id:?} ({id_str}) in index!");
        return;
    };

    entries_modrinth.insert(id_str, config.clone());

    for dep in &config.dependencies {
        add_downloaded_mod_to_entries(entries_modrinth, index, &ModId::from_index_str(dep));
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
        let accumulation = accumulation.join(path.file_name().unwrap());
        let acc_name = accumulation.to_string_lossy();

        if path.is_dir() {
            zip.add_directory(
                format!("{acc_name}/"),
                zip::write::FileOptions::<()>::default(),
            )
            .map_err(ModError::Zip)?;

            // ... accumulation = "config/dir1"
            // Then this call will have "config/dir1" as starting value.
            Box::pin(add_dir_to_zip_recursive(&path, zip, accumulation.clone())).await?;
        } else {
            // ... accumulation = "config/file1.txt"
            let bytes = tokio::fs::read(&path).await.path(path.clone())?;

            zip.start_file(&acc_name, zip::write::FileOptions::<()>::default())?;
            zip.write_all(&bytes)
                .map_err(|n| ModError::ZipIoError(n, acc_name.to_string()))?;
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
