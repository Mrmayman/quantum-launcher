use std::path::{Path, PathBuf};

use ql_core::file_utils::get_launcher_dir;

use crate::import_export::import::{self, get_instances_path, InstanceInfo};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, Write,Seek,Read};

use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
fn instance_version_finder(instance_name: String) -> Result<String, Box<dyn Error>> {
    // Get the base instance path
    let base_path = import::get_instances_path()?;
    let versions_path = base_path
        .join(&instance_name)
        .join(".minecraft")
        .join("versions");

    println!("Looking for versions in: {:?}", versions_path);

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


fn instance_info_creater(instance_name: String) -> Result<import::InstanceInfo, Box<dyn std::error::Error>>{
    let instance_version = instance_version_finder(instance_name.clone())?;
    let exeption = vec![String::from(".minecraft/versions"),String::from("libraries/natives/")];
    Ok(InstanceInfo { instance_name: instance_name, instance_version: instance_version, exeption: exeption })

}

//exeption is for implemnting selecting export 


fn merge_exeption(user_option: Option<Vec<String>>, instance: &InstanceInfo) -> Option<Vec<String>> {
    let mut result = instance.exeption.clone();

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


fn write_instance_json(instance_info: &InstanceInfo, dest_dir: &Path) -> std::io::Result<()> {
    let json_array = vec![instance_info];

    let json_path = dest_dir.join("quantum-config.json");

    let file = File::create(json_path)?;
    serde_json::to_writer_pretty(file, &json_array)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn delete_exceptions(exceptions: &[String],temp_path: &Path) -> std::io::Result<()> {
    for rel_path in exceptions {
        let full_path = temp_path.join(rel_path);


        if full_path.exists() {
            if full_path.is_dir() {
                println!("Deleting directory: {:?}", full_path);
                fs::remove_dir_all(&full_path)?;
            } else if full_path.is_file() {
                println!("Deleting file: {:?}", full_path);
                fs::remove_file(&full_path)?;
            } else {
                // Could be a symlink or something else, attempt remove_file first, fallback to remove_dir_all
                println!("Deleting special file or symlink: {:?}", full_path);
                match fs::remove_file(&full_path) {
                    Ok(_) => {},
                    Err(_) => {
                        fs::remove_dir_all(&full_path)?;
                    }
                }
            }
        } else {
            println!("Path not found, skipping: {:?}", full_path);
        }
    }
    Ok(())
}


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
        let name_in_zip = name_in_zip.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid path encoding")
        })?;

        if path.is_file() {
            println!("Adding file: {}", name_in_zip);
            zip.start_file(name_in_zip, options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !relative_path.as_os_str().is_empty() {
            println!("Adding directory: {}", name_in_zip);
            zip.add_directory(name_in_zip, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}


pub fn export_instance(instance_config: import::InstanceInfo , destination:  PathBuf,exeption: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>>{

    let exporting_version = instance_version_finder(instance_config.instance_name.clone())?;
    println!("{}",exporting_version);
    let mut export_config = instance_info_creater(instance_config.instance_name)?;
    println!("{:?}",export_config);
    let exeption_vector = merge_exeption(exeption, &export_config).expect("Couldnt find .minecraft/version and libraries/natives in the vector");
    export_config.exeption = exeption_vector;
    println!("{:?}",export_config.exeption);
    let instance_path = get_instances_path()?.join(export_config.instance_name.clone());
    println!("{:?}",instance_path);
    copy_instance_to_temp(&instance_path)?;
    let temp_instance_path: std::path::PathBuf = get_launcher_dir()?.join("temp").join(&export_config.instance_name);
    println!("{:?}",temp_instance_path);
    write_instance_json(&export_config, &temp_instance_path);

    delete_exceptions(&export_config.exeption,&temp_instance_path);
    let mut output_zip_name = export_config.instance_name.clone();
    output_zip_name.push_str(".zip");
    zip_folder(temp_instance_path, destination.join(output_zip_name),&export_config.instance_name)?;



    Ok(())
}