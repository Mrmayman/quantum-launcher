use std::path::Path;

use include_dir::{include_dir, Dir};
use ql_core::{file_utils, IntoIoError, IoError};

static PLUGINS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../plugins/");

pub async fn install_plugins() -> Result<(), IoError> {
    let path = file_utils::get_launcher_dir().await?.join("plugins");
    tokio::fs::create_dir_all(&path).await.path(&path)?;

    install_dir(&path, &PLUGINS_DIR).await?;

    Ok(())
}

async fn install_dir<'a>(path: &'a Path, dir: &'a Dir<'static>) -> Result<(), IoError> {
    for entry in dir.entries() {
        let full_path = path.parent().unwrap().join(entry.path());
        match entry {
            include_dir::DirEntry::Dir(dir) => {
                tokio::fs::create_dir_all(&full_path)
                    .await
                    .path(full_path)?;
                Box::pin(install_dir(&path.join(dir.path()), dir)).await?
            }
            include_dir::DirEntry::File(file) => {
                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(&parent).await.path(&full_path)?;
                }

                if !full_path.exists() {
                    tokio::fs::write(&full_path, file.contents())
                        .await
                        .path(full_path)?;
                }
            }
        }
    }
    Ok(())
}
