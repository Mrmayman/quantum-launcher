
use std::fs::File;
use std::path::Path;
use zip_extract::extract;
use std::env;
use std::path::PathBuf;



fn get_linux_instances_path() -> PathBuf {
    let home_dir = env::var("HOME").expect("HOME env variable not set");
    let mut path = PathBuf::from(home_dir);
    path.push(".config");
    path.push("QuantumLauncher");
    path.push("instances");
    path

}

pub fn import_instance_linux(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let instances_dir = get_linux_instances_path();

    std::fs::create_dir_all(&instances_dir)?;

    let zip_file = File::open(zip_path)?;

    extract(zip_file, &instances_dir,false)?;

    println!("Instance imported to {}", instances_dir.display());
    Ok(())
}



// pub fn full_import(import_path: &Path)  {
    
// }


