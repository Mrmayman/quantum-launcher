use ql_core::{file_utils, GenericProgress};
use ql_core::{info, pt, InstanceSelection, IntoIoError, IntoJsonError};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use tokio::fs;

use crate::{InstanceInfo, InstancePackageError};

pub const EXCEPTIONS: &[&str] = &[
    ".minecraft/versions",
    ".minecraft/usercache.json",
    "libraries",
    "fabric.json",
    "forge",
];

fn create_instance_info(
    instance: &InstanceSelection,
    mut exceptions: HashSet<String>,
) -> InstanceInfo {
    exceptions.extend(EXCEPTIONS.iter().map(|n| (*n).to_owned()));
    InstanceInfo {
        instance_name: instance.get_name().to_owned(),
        exceptions,
        is_server: instance.is_server(),
    }
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
/// 2. Constructs a new `InstanceInfo` with exceptions.
/// 3. Copies the instance files into a temporary directory.
/// 4. Writes the `InstanceInfo` to a `quantum-config.json` inside temp folder.
/// 5. Deletes the excluded directories/files from the temp copy.
/// 6. Compresses the temp folder into a `.zip` archive at the given destination.
///
/// # Errors
///
/// Returns an error if:
/// - The instance version can't be found.
/// - The instance directory doesn't exist.
/// - File I/O operations (copying, deleting, zipping) fail.
pub async fn export_instance(
    instance: InstanceSelection,
    exceptions: HashSet<String>,
    progress: Option<Sender<GenericProgress>>,
) -> Result<Vec<u8>, InstancePackageError> {
    info!("Exporting instance...");
    let export_config = create_instance_info(&instance, exceptions);
    // println!("{:?}",export_config);

    pt!(
        "Exceptions (not included in export): {:?}",
        export_config.exceptions
    );
    if let Some(prog) = &progress {
        _ = prog.send(GenericProgress {
            done: 0,
            total: 2,
            message: Some("Copying data...".to_owned()),
            has_finished: false,
        });
    }
    let dir = tempfile::TempDir::new().map_err(InstancePackageError::TempDir)?;
    let instance_path = instance.get_instance_path();
    let collect: Vec<PathBuf> = export_config
        .exceptions
        .iter()
        .map(|n| instance_path.join(n))
        .collect();
    file_utils::copy_dir_recursive_ext(&instance_path, dir.path(), &collect).await?;
    let folder_path = dir.path();

    // pt!("{:?}",temp_instance_path);
    pt!("Creating metadata");
    let config = serde_json::to_string_pretty(&export_config).json_to()?;
    let config_path = folder_path.join("quantum-config.json");
    fs::write(&config_path, config).await.path(&config_path)?;

    pt!("Packaging the instance into zip");
    if let Some(prog) = &progress {
        _ = prog.send(GenericProgress {
            done: 1,
            total: 2,
            message: Some("Zipping files...".to_owned()),
            has_finished: false,
        });
    }
    let bytes = file_utils::zip_directory_to_bytes(folder_path)
        .await
        .map_err(InstancePackageError::ZipIo)?;
    pt!("Done!");

    Ok(bytes)
}
