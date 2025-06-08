// use std::fs::File;
// use std::io::{self, Write};
// use std::path::{Path, PathBuf};
// use ql_core::print;
// use zip::write::{FileOptions, ExtendedFileOptions};
// use walkdir::WalkDir;
// use ql_core::file_utils::get_launcher_dir;

// fn get_instances_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
//     let launcher_dir = get_launcher_dir()?;
//     Ok(launcher_dir.join("instances"))
// }

// pub fn export_instance(instance_name: &str, export_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
//     let instances_dir = get_instances_path()?;
//     let instance_path = instances_dir.join(instance_name);
//     println!("{} , {:?}",instance_name,instance_path);
//     if !instance_path.exists() {
//         return Err(format!("Instance '{}' does not exist", instance_name).into());
//     }
//     let file = File::create(export_path)?;
//     let mut zip = zip::ZipWriter::new(file);

//     let options: FileOptions<ExtendedFileOptions> = FileOptions::default()
//         .compression_method(zip::CompressionMethod::Deflated)
//         .unix_permissions(0o755);

//     for entry in WalkDir::new(&instance_path) {
//         println!("reached here");
//         let entry = entry?;
//         let path = entry.path();
//         let name = path.strip_prefix(&instances_dir)?;
//         // used clone it may impact perfomance 
//         if path.is_file() {
//             zip.start_file(name.to_string_lossy(), options.clone())?;
//             let mut f = File::open(path)?;
//             io::copy(&mut f, &mut zip)?;
//         } else if name.as_os_str().len() != 0 {
//             zip.add_directory(name.to_string_lossy(), options.clone())?;
//         }
//     }

//     zip.finish()?;
//     println!("Instance '{}' exported to {}", instance_name, export_path.display());
//     Ok(())
// }

