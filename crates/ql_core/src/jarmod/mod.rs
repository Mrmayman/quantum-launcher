use std::path::{Path, PathBuf, StripPrefixError};

use crate::{
    get_jar_path,
    json::{JsonOptifine, VersionDetails},
    pt, InstanceSelection, IntoIoError, IoError, JsonError, JsonFileError,
};
use thiserror::Error;
use zip_dir::zip_directory_to_bytes;
use zip_extract::ZipExtractError;

mod json;
mod zip_dir;

pub use json::{JarMod, JarMods};

pub async fn remove(instance: &InstanceSelection, filename: &str) -> Result<(), JsonFileError> {
    let mut jarmods = JarMods::get(instance).await?;

    if let Some(idx) = jarmods
        .mods
        .iter()
        .enumerate()
        .find_map(|n| (n.1.filename == filename).then_some(n.0))
    {
        jarmods.mods.remove(idx);
    }

    let mod_path = instance.get_instance_path().join("jarmods").join(filename);
    if mod_path.is_file() {
        tokio::fs::remove_file(&mod_path).await.path(&mod_path)?;
    }

    jarmods.save(instance).await?;
    Ok(())
}

pub async fn insert(instance: InstanceSelection, bytes: Vec<u8>) -> Result<(), JsonFileError> {
    let mut jarmods = JarMods::get(&instance).await?;

    let jarmods_dir = instance.get_instance_path().join("jarmods");
    if !jarmods_dir.is_dir() {
        tokio::fs::create_dir_all(&jarmods_dir)
            .await
            .path(&jarmods_dir)?;
    }

    let file_path = jarmods_dir.join("Optifine.zip");
    tokio::fs::write(&file_path, &bytes)
        .await
        .path(&file_path)?;

    jarmods.mods.push(JarMod {
        filename: "Optifine.zip".to_owned(),
        enabled: true,
    });
    jarmods.save(&instance).await?;

    Ok(())
}

pub async fn build(instance: &InstanceSelection) -> Result<PathBuf, JarModError> {
    let instance_dir = instance.get_instance_path();
    let jarmods_dir = instance_dir.join("jarmods");
    tokio::fs::create_dir_all(&jarmods_dir)
        .await
        .path(&jarmods_dir)?;

    let json = VersionDetails::load(instance).await?;
    let optifine = JsonOptifine::read(instance.get_name()).await.ok();

    let original_jar_path = get_jar_path(
        &json,
        &instance_dir,
        optifine.as_ref().map(|n| n.1.as_path()),
    );

    if is_dir_empty(&jarmods_dir).await {
        return Ok(original_jar_path);
    }

    let index = JarMods::get(instance).await?;

    let tmp_dir = jarmods_dir.join("tmp");
    tokio::fs::create_dir_all(&tmp_dir).await.path(&tmp_dir)?;

    let original_jar_bytes = tokio::fs::read(&original_jar_path)
        .await
        .path(&original_jar_path)?;
    zip_extract::extract(std::io::Cursor::new(&original_jar_bytes), &tmp_dir, true)?;

    for jar in &index.mods {
        if !jar.enabled {
            continue;
        }

        pt!("{}", jar.filename);
        let path = jarmods_dir.join(&jar.filename);
        let bytes = tokio::fs::read(&path).await.path(&path)?;
        zip_extract::extract(std::io::Cursor::new(&bytes), &tmp_dir, true)?;
    }

    let meta_inf = tmp_dir.join("META-INF");
    if meta_inf.is_dir() {
        tokio::fs::remove_dir_all(&meta_inf).await.path(&meta_inf)?;
    }

    let zip = zip_directory_to_bytes(&tmp_dir).await?;
    let out_jar = instance_dir.join("build.jar");
    tokio::fs::write(&out_jar, &zip).await.path(&out_jar)?;

    tokio::fs::remove_dir_all(&tmp_dir).await.path(&tmp_dir)?;

    Ok(out_jar)
}

pub async fn is_dir_empty(path: &Path) -> bool {
    let Ok(mut dir) = tokio::fs::read_dir(path).await else {
        return false;
    };
    dir.next_entry().await.ok().flatten().is_none()
}

const JARMOD_ERR_PREFIX: &str = "while dealing with jar mod:\n";

#[derive(Error, Debug)]
pub enum JarModError {
    #[error("{JARMOD_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{JARMOD_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{JARMOD_ERR_PREFIX}while walking through dir:\n{0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("{JARMOD_ERR_PREFIX}while stripping prefix of jarmods/tmp:\n{0}")]
    StripPrefix(#[from] StripPrefixError),

    #[error("{JARMOD_ERR_PREFIX}while extracting zip:\n{0}")]
    ZipExtract(#[from] ZipExtractError),
    #[error("{JARMOD_ERR_PREFIX}while processing zip:\n{0}")]
    ZipError(#[from] ::zip::result::ZipError),
    #[error("{JARMOD_ERR_PREFIX}while reading from zip:\n{0}")]
    ZipWriteError(std::io::Error),
}

impl From<JsonFileError> for JarModError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => Self::Json(err),
            JsonFileError::Io(err) => Self::Io(err),
        }
    }
}
