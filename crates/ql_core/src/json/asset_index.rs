use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::{err, file_utils, DownloadFileError, IntoIoError, RequestError};

#[derive(Serialize, Deserialize)]
pub struct AssetIndexMap {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl AssetObject {
    pub async fn download(&self, objects_path: &Path) -> Result<(), DownloadFileError> {
        const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

        let obj_id = &self.hash[0..2];

        let obj_folder = objects_path.join(obj_id);
        tokio::fs::create_dir_all(&obj_folder)
            .await
            .path(&obj_folder)?;

        let obj_file_path = obj_folder.join(&self.hash);
        if obj_file_path.exists() {
            return Ok(());
        }

        let url = self
            .url
            .clone()
            .unwrap_or(format!("{OBJECTS_URL}/{obj_id}/{}", self.hash));
        let err = file_utils::download_file_to_path(&url, false, &obj_file_path).await;

        match err {
            Ok(()) => {}
            Err(DownloadFileError::Request(RequestError::DownloadError { code, .. }))
                if code.as_u16() == 404 =>
            {
                err!("Error 404 for asset: {url}, skipping...");
            }
            Err(err) => Err(err)?,
        }

        Ok(())
    }
}
