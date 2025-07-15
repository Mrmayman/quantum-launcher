use crate::{err, file_utils, IntoJsonError, IoError};
use crate::{InstanceSelection, IntoIoError, JsonFileError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JarMods {
    pub mods: Vec<JarMod>,
}

impl JarMods {
    pub async fn get(instance: &InstanceSelection) -> Result<Self, JsonFileError> {
        let path = instance.get_instance_path().join("jarmods.json");

        if path.is_file() {
            let file = tokio::fs::read_to_string(&path).await.path(path)?;
            let file = serde_json::from_str(&file).json(file)?;
            Ok(file)
        } else {
            let file = Self { mods: Vec::new() };
            let file_str = serde_json::to_string(&file).json_to()?;
            tokio::fs::write(&path, &file_str).await.path(&file_str)?;
            Ok(file)
        }
    }

    pub fn get_s(instance: &InstanceSelection) -> Result<Self, JsonFileError> {
        let path = instance.get_instance_path().join("jarmods.json");

        if path.is_file() {
            let file = std::fs::read_to_string(&path).path(path)?;
            let file = serde_json::from_str(&file).json(file)?;
            Ok(file)
        } else {
            let file = Self { mods: Vec::new() };
            let file_str = serde_json::to_string(&file).json_to()?;
            std::fs::write(&path, &file_str).path(&file_str)?;
            Ok(file)
        }
    }

    pub async fn save(&mut self, instance: &InstanceSelection) -> Result<(), JsonFileError> {
        self.trim(instance);
        if let Err(err) = self.expand(instance).await {
            err!("While expanding jarmods.json with new entries: {err}");
        }

        let path = instance.get_instance_path().join("jarmods.json");
        let file = serde_json::to_string(self).json_to()?;
        tokio::fs::write(&path, &file).await.path(file)?;
        Ok(())
    }

    fn trim(&mut self, instance: &InstanceSelection) {
        let path = instance.get_instance_path().join("jarmods");
        self.mods.retain(|n| path.join(&n.filename).is_file());
    }

    pub async fn expand(&mut self, instance: &InstanceSelection) -> Result<(), IoError> {
        let path = instance.get_instance_path().join("jarmods");
        if !path.is_dir() {
            tokio::fs::create_dir_all(&path).await.path(path)?;
            return Ok(());
        }
        let filenames = file_utils::read_filenames_from_dir(&path).await?;

        let existing_filenames: std::collections::HashSet<_> =
            self.mods.iter().map(|m| m.filename.clone()).collect();

        self.mods.extend(
            filenames
                .into_iter()
                .filter(|f| !existing_filenames.contains(f))
                .map(|filename| JarMod {
                    filename,
                    enabled: true,
                }),
        );

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JarMod {
    pub filename: String,
    pub enabled: bool,
}
