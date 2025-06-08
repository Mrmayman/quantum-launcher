use std::path::{Path};

use crate::import_export::import::{self, InstanceInfo};
use std::error::Error;
use std::fs;

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

pub fn export_instance(instance_config: import::InstanceInfo , destination:  &Path,exeption: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>>{

    let x = instance_version_finder(instance_config.instance_name.clone())?;
    println!("{}",x);
    let config = instance_info_creater(instance_config.instance_name)?;
    println!("{:?}",config);
    Ok(())
}