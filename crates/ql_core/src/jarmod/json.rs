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
            let file = serde_json::from_str(&file)?;
            Ok(file)
        } else {
            let file = Self { mods: Vec::new() };
            let file_str = serde_json::to_string(&file)?;
            println!("{path:?}");
            tokio::fs::write(&path, &file_str).await.path(&file_str)?;
            Ok(file)
        }
    }

    pub async fn save(&mut self, instance: &InstanceSelection) -> Result<(), JsonFileError> {
        self.trim(instance).await;

        let path = instance.get_instance_path().join("jarmods.json");
        let file = serde_json::to_string(self)?;
        tokio::fs::write(&path, &file).await.path(file)?;
        Ok(())
    }

    async fn trim(&mut self, instance: &InstanceSelection) {
        let path = instance.get_instance_path().join("jarmods");
        self.mods.retain(|n| {
            path.join(&n.filename).is_file()
                || path.join(format!("{}.disabled", n.filename)).is_file()
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JarMod {
    pub filename: String,
    pub enabled: bool,
}
