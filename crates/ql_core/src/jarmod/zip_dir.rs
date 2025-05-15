use std::{
    io::{Cursor, Write},
    path::Path,
};

use crate::IntoIoError;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

use super::JarModError;

pub async fn zip_directory_to_bytes<P: AsRef<Path>>(dir: P) -> Result<Vec<u8>, JarModError> {
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let dir = dir.as_ref();
    let base_path = dir;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(base_path)?;
            let name_in_zip = relative_path.to_string_lossy().replace('\\', "/"); // For Windows compatibility

            zip.start_file(name_in_zip, options)?;
            let bytes = tokio::fs::read(path).await.path(path)?;
            zip.write_all(&bytes).map_err(JarModError::ZipWriteError)?;
        }
    }

    zip.finish()?;
    Ok(buffer.into_inner())
}
