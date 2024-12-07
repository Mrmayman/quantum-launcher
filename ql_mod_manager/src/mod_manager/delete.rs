use crate::mod_manager::ModError;
use crate::mod_manager::ModIndex;
use ql_instances::err;
use ql_instances::error::IoError;
use ql_instances::file_utils;
use ql_instances::info;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

pub async fn delete_mods_wrapped(
    id: Vec<String>,
    instance_name: String,
) -> Result<Vec<String>, String> {
    delete_mods(&id, &instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| id)
}

pub async fn delete_mods(id: &[String], instance_name: &str) -> Result<(), ModError> {
    info!("Deleting mods: {{");
    let mut index = ModIndex::get(instance_name)?;

    let launcher_dir = file_utils::get_launcher_dir()?;
    let mods_dir = launcher_dir
        .join("instances")
        .join(instance_name)
        .join(".minecraft/mods");

    // let mut downloaded_mods = HashSet::new();

    for id in id {
        println!("- Deleting mod {id}");
        delete_mod(&mut index, id, &mods_dir)?;
        // delete_item(id, None, &mut index, &mods_dir, &mut downloaded_mods)?;
    }

    let mut has_been_removed;
    let mut iteration = 0;
    loop {
        iteration += 1;
        has_been_removed = false;
        println!("- Iteration {iteration}");
        let mut removed_dependents_map = HashMap::new();

        for (mod_id, mod_info) in &index.mods {
            if !mod_info.manually_installed {
                let mut removed_dependents = HashSet::new();
                for dependent in &mod_info.dependents {
                    if !index.mods.contains_key(dependent) {
                        removed_dependents.insert(dependent.clone());
                    }
                }
                removed_dependents_map.insert(mod_id.clone(), removed_dependents);
            }
        }

        for (id, removed_dependents) in removed_dependents_map {
            if let Some(mod_info) = index.mods.get_mut(&id) {
                for dependent in removed_dependents {
                    has_been_removed = true;
                    mod_info.dependents.remove(&dependent);
                }
            } else {
                err!("Dependent {id} does not exist")
            }
        }

        let mut orphaned_mods = HashSet::new();

        for (mod_id, mod_info) in &index.mods {
            if !mod_info.manually_installed && mod_info.dependents.is_empty() {
                println!("- Deleting child {}", mod_info.name);
                orphaned_mods.insert(mod_id.clone());
            }
        }

        for orphan in orphaned_mods {
            has_been_removed = true;
            delete_mod(&mut index, &orphan, &mods_dir)?;
        }

        if !has_been_removed {
            break;
        }
    }

    index.save()?;
    println!("}} Done deleting mods");
    Ok(())
}

fn delete_mod(index: &mut ModIndex, id: &String, mods_dir: &Path) -> Result<(), ModError> {
    if let Some(mod_info) = index.mods.remove(id) {
        for file in &mod_info.files {
            if mod_info.enabled {
                delete_file(mods_dir, &file.filename)?;
            } else {
                delete_file(mods_dir, &format!("{}.disabled", file.filename))?;
            }
        }
    } else {
        err!("Deleted mod does not exist");
    }
    Ok(())
}

fn delete_file(mods_dir: &Path, file: &str) -> Result<(), ModError> {
    let path = mods_dir.join(file);
    if let Err(err) = std::fs::remove_file(&path) {
        if let std::io::ErrorKind::NotFound = err.kind() {
            eprintln!("[warning] File does not exist, skipping: {path:?}");
        } else {
            let err = IoError::Io {
                error: err,
                path: path.clone(),
            };
            Err(err)?;
        }
    }
    Ok(())
}

// pub async fn delete_mod_wrapped(id: String, instance_name: String) -> Result<String, String> {
//     delete_mod(&id, instance_name)
//         .await
//         .map_err(|err| err.to_string())
//         .map(|()| id)
// }

// pub async fn delete_mod(id: &str, instance_name: String) -> Result<(), ModrinthError> {
//     let mut index = ModIndex::get(&instance_name)?;
//     let mut downloaded_mods = HashSet::new();

//     let launcher_dir = file_utils::get_launcher_dir()?;
//     let mods_dir = launcher_dir
//         .join("instances")
//         .join(&instance_name)
//         .join(".minecraft/mods");
//     delete_item(id, None, &mut index, &mods_dir, &mut downloaded_mods)?;

//     index.save()?;
//     Ok(())
// }

// fn delete_item(
//     id: &str,
//     parent: Option<&str>,
//     index: &mut ModIndex,
//     mods_dir: &Path,
//     downloaded_mods: &mut HashSet<String>,
// ) -> Result<(), ModrinthError> {
//     info!("Deleting mod {id}");
//     let already_deleted = !downloaded_mods.insert(id.to_owned());
//     if already_deleted {
//         println!("- Already deleted, skipping");
//         return Ok(());
//     }

//     if let Some(mod_info) = index.mods.get_mut(id) {
//         if let Some(parent) = parent {
//             mod_info.dependents = mod_info
//                 .dependents
//                 .iter()
//                 .filter_map(|n| {
//                     println!("dependent {n} : parent {parent}");
//                     if n.as_str() == parent {
//                         None
//                     } else {
//                         Some(n.clone())
//                     }
//                 })
//                 .collect();

//             println!("id {id} : dependents {:?}", mod_info.dependents);
//             if !mod_info.dependents.is_empty() {
//                 return Ok(());
//             }
//         }
//     } else {
//         eprintln!("[warning] Could not find mod in index: {id}");
//     }
//     if let Some(mod_info) = index.mods.get(id).cloned() {
//         for file in &mod_info.files {
//             let path = mods_dir.join(&file.filename);
//             if let Err(err) = std::fs::remove_file(&path) {
//                 if let std::io::ErrorKind::NotFound = err.kind() {
//                     eprintln!("[warning] File does not exist, skipping: {path:?}");
//                 } else {
//                     let err = IoError::Io {
//                         error: err,
//                         path: path.to_owned(),
//                     };
//                     Err(err)?;
//                 }
//             }
//         }

//         for dependency in &mod_info.dependencies {
//             delete_item(dependency, Some(id), index, mods_dir, downloaded_mods)?;
//         }
//     }
//     index.mods.remove(id);
//     Ok(())
// }
