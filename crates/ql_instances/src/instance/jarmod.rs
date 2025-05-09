use std::path::{Path, PathBuf};

use ql_core::{
    json::{JsonOptifine, VersionDetails},
    InstanceSelection, IntoIoError, IoError, JsonFileError,
};
use thiserror::Error;

use crate::instance::launch::GameLauncher;

#[allow(dead_code)] // incomplete
pub async fn build(instance: &InstanceSelection) -> Result<PathBuf, JarModError> {
    let instance_dir = instance.get_instance_path();
    let jarmods_dir = instance_dir.join("jarmods");
    tokio::fs::create_dir_all(&jarmods_dir)
        .await
        .path(&jarmods_dir)?;

    let json = VersionDetails::load(instance).await?;
    let optifine = JsonOptifine::read(instance.get_name()).await.ok();

    let original_jar_path = GameLauncher::get_jar_path(
        &json,
        &instance_dir,
        optifine.as_ref().map(|n| n.1.as_path()),
    );

    if is_dir_empty(&jarmods_dir).await {
        return Ok(original_jar_path);
    }

    todo!()
}

pub async fn is_dir_empty(path: &Path) -> bool {
    let Ok(mut dir) = tokio::fs::read_dir(path).await else {
        return false;
    };
    dir.next_entry().await.ok().flatten().is_none()
}

#[derive(Error, Debug)]
pub enum JarModError {
    #[error("jar mod: {0}")]
    Io(#[from] IoError),
    #[error("jar mod: json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<JsonFileError> for JarModError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => Self::Json(err),
            JsonFileError::Io(err) => Self::Io(err),
        }
    }
}
