use ql_core::file_utils;
use ql_core::{InstanceSelection, IntoIoError, IntoJsonError, IoError, info, pt};
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

use crate::{InstanceInfo, InstancePackageError};

pub const EXCEPTIONS: &[&str] = &[
    ".minecraft/versions",
    ".minecraft/usercache.json",
    "libraries",
];

fn create_instance_info(
    instance: &InstanceSelection,
    mut exceptions: HashSet<String>,
) -> Result<InstanceInfo, InstancePackageError> {
    exceptions.extend(EXCEPTIONS.iter().map(|n| (*n).to_owned()));
    Ok(InstanceInfo {
        instance_name: instance.get_name().to_owned(),
        exceptions: HashSet::new(),
        is_server: instance.is_server(),
    })
}

// deltes the folders present in the exceptions
async fn delete_exceptions(
    exceptions: &HashSet<String>,
    temp_path: &Path,
) -> Result<(), InstancePackageError> {
    for rel_path in exceptions {
        let full_path = temp_path.join(rel_path);
        if !full_path.starts_with(temp_path) {
            // If the exception "../../../../etc/passwd" or something
            // then avoid SECURITY RISK
            return Err(InstancePackageError::Io(IoError::DirEscapeAttack));
        }

        if full_path.is_dir() {
            pt!("Deleting directory: {:?}", full_path);
            fs::remove_dir_all(&full_path).await.path(&full_path)?;
        } else if full_path.is_file() {
            pt!("Deleting file: {:?}", full_path);
            fs::remove_file(&full_path).await.path(&full_path)?;
        } else {
            pt!("Path not found, skipping: {:?}", full_path);
        }
    }
    Ok(())
}

/// Exports a Minecraft instance by copying its files to a temporary directory,
/// removing specified exceptions, generating a metadata JSON, and zipping the result.
///
/// # Arguments
///
/// - `instance` - the selected instance to export
/// - `exceptions` - An optional vector of paths to exclude from the export.
///   By default it will contain the contents of [`EXCEPTIONS`]. If you
///   don't want any extra exceptions, just pass an empty `Vec`.
///   **Note: All exception paths are relative to instance dir
///   (parent dir of `.minecraft`)**
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` (bytes of the packaged file)
/// if the export succeeds, or an error if any step fails.
///
/// # Process
///
/// 1. Detects the version of the given instance.
/// 2. Constructs a new `InstanceInfo` with merged exceptions.
/// 3. Copies the instance files into a temporary directory.
/// 4. Writes a `quantum-config.json` metadata file inside the temp folder.
/// 5. Deletes the excluded directories/files from the temp copy.
/// 6. Compresses the temp folder into a `.zip` archive at the given destination.
///
/// # Errors
///
/// Returns an error if:
/// - The instance version can't be found.
/// - The instance directory doesn't exist.
/// - File I/O operations (copying, deleting, zipping) fail.
/// - The `exception` vector is missing critical paths (`.minecraft/versions`, `libraries/natives`).
///
/// # Example
///
/// ```rust
/// let info = InstanceInfo {
///     instance_name: "MyInstance".to_string(),
///     instance_version: "1.20.4".to_string(),
///     exception: vec![],
/// };
/// export_instance(info, PathBuf::from("exports/"), None)?;
/// ```
pub async fn export_instance(
    instance: InstanceSelection,
    exceptions: HashSet<String>,
) -> Result<Vec<u8>, InstancePackageError> {
    info!("Exporting instance...");
    let export_config = create_instance_info(&instance, exceptions)?;
    // println!("{:?}",export_config);

    pt!(
        "Exceptions (not included in export): {:?}",
        export_config.exceptions
    );
    let dir = tempdir::TempDir::new("ql_instance_export").map_err(InstancePackageError::TempDir)?;
    file_utils::copy_dir_recursive(&instance.get_instance_path(), dir.path()).await?;
    let folder_path = dir.path();

    // pt!("{:?}",temp_instance_path);
    pt!("Creating metadata");
    let config = serde_json::to_string_pretty(&export_config).json_to()?;
    let config_path = folder_path.join("quantum-config.json");
    fs::write(&config_path, config).await.path(&config_path)?;

    pt!("Deleting exceptions");
    delete_exceptions(&export_config.exceptions, folder_path).await?;

    pt!("Packaging the instance into zip");
    let bytes = file_utils::zip_directory_to_bytes(folder_path)
        .await
        .map_err(InstancePackageError::ZipIo)?;

    Ok(bytes)
}
