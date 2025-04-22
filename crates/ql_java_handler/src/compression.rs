use std::{io::Cursor, path::Path};

use flate2::read::GzDecoder;
use tar::Archive;

/// Extracts a `.tar.gz` file from a `&[u8]` buffer into the given directory.
/// Does not create a top-level directory,
/// extracting files directly into the target directory.
///
/// # Arguments
/// - `data`: A reference to the `.tar.gz` file as a byte slice.
/// - `output_dir`: Path to the directory where the contents will be extracted.
///
/// # Errors
/// - `std::io::Error` if the `.tar.gz` file was invalid.
pub fn extract_tar_gz(archive: &[u8], output_dir: &Path) -> std::io::Result<()> {
    // For extracting the `.gz`
    let decoder = GzDecoder::new(Cursor::new(archive));
    // For extracting the `.tar`
    let mut tar = Archive::new(decoder);

    // Get the first entry path to determine the top-level directory
    let mut entries = tar.entries()?;
    let top_level_dir = if let Some(entry) = entries.next() {
        let entry = entry?;
        let path = entry
            .path()?
            .components()
            .next()
            .map(|c| c.as_os_str().to_os_string());
        path
    } else {
        None
    };

    // Rewind the archive to process all entries
    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut tar = Archive::new(decoder);

    // Extract files while flattening the top-level directory
    for entry in tar.entries()? {
        let mut entry = entry?;

        // Get the path of the file in the archive
        let entry_path = entry.path()?;

        // Remove the top-level directory from the path
        let new_path = match top_level_dir.as_ref() {
            Some(top_level) if entry_path.starts_with(top_level) => entry_path
                .strip_prefix(top_level)
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Could not strip prefix {entry_path:?}, {top_level:?}"),
                    )
                })?
                .to_path_buf(),
            _ => entry_path.to_path_buf(),
        };

        // Resolve the full output path
        let full_path = output_dir.join(new_path);

        // Ensure parent directories exist
        if let Some(parent) = full_path.parent() {
            // Not using async due to some weird thread safety error
            std::fs::create_dir_all(parent)?;
        }

        // Unpack the file or directory
        entry.unpack(full_path)?;
    }

    Ok(())
}
