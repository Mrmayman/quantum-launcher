/// Imports a QuantumLauncher instance from a ZIP archive.
///
/// This function extracts the contents of the specified ZIP file into
/// the `instances` directory inside the QuantumLauncher root folder,
/// creating the directory if it does not exist.
///
/// # Arguments
///
/// * `zip_path` - A reference to the path of the ZIP file to import.
///
/// # Errors
///
/// Returns an error if the ZIP file cannot be opened, the extraction
/// fails, or the instances directory cannot be created.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// 
/// let zip_path = Path::new("path/to/instance.zip");
/// ql_instances::import_instance(zip_path).expect("Failed to import instance");
/// ```

use std::fs::File;
use std::path::Path;
use zip_extract::extract;
use std::path::PathBuf;
use ql_core::file_utils::get_launcher_dir;

fn get_instances_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let launcher_dir = get_launcher_dir()?;
    Ok(launcher_dir.join("instances"))
}


pub fn import_instance(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let instances_dir = get_instances_path()?;
    std::fs::create_dir_all(&instances_dir)?;

    let zip_file = File::open(zip_path)?;
    extract(zip_file, &instances_dir, false)?;

    println!("Instance imported to {}", instances_dir.display());
    Ok(())
}

