use ql_core::file_utils::get_launcher_dir;
use ql_core::{info, pt};
/// Exports a Minecraft instance by copying its files to a temporary directory,
/// removing specified exceptions, generating a metadata JSON, and zipping the result.
///
/// # Arguments
///
/// * `instance_config` - An `InstanceInfo` struct that contains the instance name and metadata.
/// * `destination` - The output path where the ZIP file should be saved.
/// * `exception` - An optional vector of additional paths to exclude from the export.
/// * exception takes an optional vector so
///  * if none is passed it will by default pass => vec![".minecraft/versions","libraries/natives/"]
///  * if vector of Strings is passed it wall added the above vector + the given vector
///  * Example vec![".minecraft/versions","libraries/natives/",".minecraft/mods",".minecraft/saves"]
///  * in the above example there was two addtional element in vector compared to the version in non
///  * NOTE THE PATHS WHICH WILL BE PASSED WILL BE AS REALATIVE PATH WITH RESPECT TO THE INSTANCE FOLDER
///
/// # Returns
///
/// Returns `Ok(())` if the export succeeds, or an error if any step fails.
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
use std::path::{Path, PathBuf};

use crate::import_export::import::{self, get_instances_path, InstanceInfo};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};

use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
fn instance_version_finder(instance_name: String) -> Result<String, Box<dyn Error>> {
    // Get the base instance path
    let base_path = import::get_instances_path()?;
    let versions_path = base_path
        .join(&instance_name)
        .join(".minecraft")
        .join("versions");

    pt!("Looking for versions in: {:?}", versions_path);

    // Ensure the directory exists
    if !versions_path.exists() || !versions_path.is_dir() {
        return Err(format!("Versions path not found: {:?}", versions_path).into());
    }

    // Scan for a folder that contains a matching .jar file
    for entry in fs::read_dir(&versions_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                let jar_path = path.join(format!("{}.jar", folder_name));
                if jar_path.exists() {
                    return Ok(folder_name.to_string());
                }
            }
        }
    }

    Err("No valid Minecraft version folder found.".into())
}

// will create an instance info when instance name is passed
fn instance_info_creater(
    instance_name: String,
) -> Result<import::InstanceInfo, Box<dyn std::error::Error>> {
    let instance_version = instance_version_finder(instance_name.clone())?;
    let exception = vec![
        String::from(".minecraft/versions"),
        String::from("libraries/natives/"),
    ];
    Ok(InstanceInfo {
        instance_name: instance_name,
        instance_version: instance_version,
        exception: exception,
    })
}

//exception is for implemnting selecting export

// to merge the user give vector to the one present in the present InstanceInfo
fn merge_exception(
    user_option: Option<Vec<String>>,
    instance: &InstanceInfo,
) -> Option<Vec<String>> {
    let mut result = instance.exception.clone();

    if let Some(mut vec) = user_option {
        result.append(&mut vec);
    }

    let has_minecraft = result.iter().any(|s| s == ".minecraft/versions");
    let has_libraries = result.iter().any(|s| s == "libraries/natives/");

    if has_minecraft && has_libraries {
        Some(result)
    } else {
        None
    }
}

fn copy_recursively(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());
            if file_type.is_dir() {
                copy_recursively(&entry.path(), &dest_path)?;
            } else {
                fs::copy(&entry.path(), &dest_path)?;
            }
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn copy_instance_to_temp(instance_path: &Path) -> io::Result<()> {
    let launcher_root = get_launcher_dir().expect("couldnt not resolve launcher dir");

    let temp_folder = launcher_root.join("temp");
    if !temp_folder.exists() {
        fs::create_dir(&temp_folder)?;
    }

    let dest_path = temp_folder.join(instance_path.file_name().unwrap());

    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)?;
    }

    copy_recursively(instance_path, &dest_path)?;

    Ok(())
}

// write the quantum-config.json to root of instance folder with info present in instance_info
fn write_instance_json(instance_info: &InstanceInfo, dest_dir: &Path) -> std::io::Result<()> {
    let json_array = vec![instance_info];

    let json_path = dest_dir.join("quantum-config.json");

    let file = File::create(json_path)?;
    serde_json::to_writer_pretty(file, &json_array)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// deltes the folders present in the exceptions
fn delete_exceptions(exceptions: &[String], temp_path: &Path) -> std::io::Result<()> {
    for rel_path in exceptions {
        let full_path = temp_path.join(rel_path);

        if full_path.exists() {
            if full_path.is_dir() {
                pt!("Deleting directory: {:?}", full_path);
                fs::remove_dir_all(&full_path)?;
            } else if full_path.is_file() {
                pt!("Deleting file: {:?}", full_path);
                fs::remove_file(&full_path)?;
            } else {
                // Could be a symlink or something else, attempt remove_file first, fallback to remove_dir_all
                pt!("Deleting special file or symlink: {:?}", full_path);
                match fs::remove_file(&full_path) {
                    Ok(_) => {}
                    Err(_) => {
                        fs::remove_dir_all(&full_path)?;
                    }
                }
            }
        } else {
            pt!("Path not found, skipping: {:?}", full_path);
        }
    }
    Ok(())
}

// packs the instance into zip
fn zip_folder<P: AsRef<Path>>(
    folder_path: P,
    output_zip: P,
    parent_folder_in_zip: &str,
) -> io::Result<()> {
    let folder_path = folder_path.as_ref();
    let output_zip = output_zip.as_ref();

    // Create the ZIP file
    let file = File::create(output_zip)?;
    let mut zip = ZipWriter::new(file);

    // Options for the files in the ZIP
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Walk through the folder recursively
    for entry in WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        // Get the relative path from the folder being zipped
        let relative_path = path.strip_prefix(folder_path).unwrap();
        // Prepend the parent folder name (e.g., "craftmine") to the path in the ZIP
        let mut name_in_zip = PathBuf::from(parent_folder_in_zip);
        name_in_zip.push(relative_path);
        let name_in_zip = name_in_zip
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid path encoding"))?;

        if path.is_file() {
            // pt!("Adding file: {}", name_in_zip);
            zip.start_file(name_in_zip, options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !relative_path.as_os_str().is_empty() {
            // pt!("Adding directory: {}", name_in_zip);
            zip.add_directory(name_in_zip, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

pub fn export_instance(
    instance_name: String,
    destination: PathBuf,
    exception: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let exporting_version = instance_version_finder(instance_name.clone())?; // find the version of instance
    info!("Exporting version : {}", exporting_version);
    let mut export_config = instance_info_creater(instance_name)?; // will create a struct to contain metadata for the instance
                                                                   // println!("{:?}",export_config); can be used for debuging
    let exception_vector = merge_exception(exception, &export_config)
        .expect("Couldnt find .minecraft/version and libraries/natives in the vector");
    export_config.exception = exception_vector;
    info!(
        "exceptions(is not included in export) :{:?}",
        export_config.exception
    );
    let instance_path = get_instances_path()?.join(export_config.instance_name.clone());
    info!("{:?}", instance_path);
    copy_instance_to_temp(&instance_path)?;
    let temp_instance_path: std::path::PathBuf = get_launcher_dir()?
        .join("temp")
        .join(&export_config.instance_name);
    // pt!("{:?}",temp_instance_path); // can be used for debugging
    info!("Metadata created");
    write_instance_json(&export_config, &temp_instance_path);
    info!("Deleteing exceptions");
    delete_exceptions(&export_config.exception, &temp_instance_path);
    let mut output_zip_name = export_config.instance_name.clone();
    output_zip_name.push_str(".zip");
    info!("Packaging the instance into zip");
    zip_folder(
        temp_instance_path,
        destination.join(output_zip_name),
        &export_config.instance_name,
    )?;
    info!("Cleaning unwanted folders");
    fs::remove_dir_all(get_launcher_dir()?.join("temp"))?; // deleteing the temporary directory

    Ok(())
}
