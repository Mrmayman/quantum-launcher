
use std::fs::File;
use std::path::Path;
use ql_core::{ListEntry};
use zip_extract::extract;
use std::path::PathBuf;
use ql_core::file_utils::get_launcher_dir;
use std::io::BufReader;
use serde::Deserialize;
use std::io;
use crate::instance;
use std::fs;




pub fn get_instances_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let launcher_dir = get_launcher_dir()?;
    Ok(launcher_dir.join("instances"))
}





// basicaly get the zip file name 
//Eg if file name is instance1.zip it will remove the .zip and will be instance1
fn get_zip_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

#[derive(Debug, Deserialize)]
pub struct InstanceInfo {
    pub instance_name: String,
    #[serde(rename = "minecraft_version")]
    pub instance_version: String,
    #[serde(rename = "exeptions")]
    pub exeption: Vec<String>,
}

// this fn will be used to extract quantum-config , and convert it to InstanceInfo
fn read_instance_from_file(path: &Path) -> Result<InstanceInfo, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let instances: Vec<InstanceInfo> = serde_json::from_reader(reader)?;
    instances.into_iter().next().ok_or_else(|| "No instance found".into())
}


fn copy_dir_recursive_overwrite(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            // Recursively copy directories
            copy_dir_recursive_overwrite(&src_path, &dst_path)?;
        } else {
            // If file exists at destination, remove it
            if dst_path.exists() {
                fs::remove_file(&dst_path)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}


pub async  fn import_instance(zip_path: &Path,assets: bool)-> Result<(), Box<dyn std::error::Error>>{
    let instances_dir = get_launcher_dir()?;
    println!("{:?}",instances_dir);
    let instance_name_zip = get_zip_stem(zip_path).unwrap(); // will change this later unwrap is unsafe for this
    // println!("{}",instance_name);
    let temp_dir = get_launcher_dir()?;
    let temp_dir = temp_dir.join("temp");
    std::fs::create_dir(&temp_dir); // creating a temproary directory for extracting zip
    
    let zip_file = File::open(zip_path)?;

    extract(zip_file, &temp_dir, false); // extracts the file to temporary directry

    println!("Instance extracted to {}", temp_dir.display());

    let config_path = temp_dir.join(instance_name_zip);
    let config_file = String::from("quantum-config.json");
    let instance_info = read_instance_from_file(&config_path.join(&config_file))?;
    println!("{:?}",instance_info);
    instance::create::create_instance(instance_info.instance_name,ListEntry { name: instance_info.instance_version, is_classic_server: false } , None, assets).await?;
    let destination = get_instances_path()?;
    println!("{:?}",destination);
    copy_dir_recursive_overwrite(&temp_dir, &destination);

    Ok(())
}
