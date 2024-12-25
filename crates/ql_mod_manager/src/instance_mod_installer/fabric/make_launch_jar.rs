use ql_core::io_err;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipWriter};

use super::error::FabricInstallError;

const MANIFEST_PATH: &str = "META-INF/MANIFEST.MF";
const SERVICES_DIR: &str = "META-INF/services/";
const MAIN_CLASS_MANIFEST: &str = "net.fabricmc.loader.launch.server.FabricServerLauncher";

pub async fn make_launch_jar(
    file: &Path,
    launch_main_class: &str,
    library_files: &[PathBuf],
    shade_libraries: bool,
) -> Result<(), FabricInstallError> {
    // Delete the output file if it already exists
    if file.exists() {
        tokio::fs::remove_file(file).await.map_err(io_err!(file))?;
    }

    // Create a new ZIP file
    let zip_file = File::create(file).map_err(io_err!(file))?;
    let mut zip_writer = ZipWriter::new(BufWriter::new(zip_file));
    let mut added_entries = HashSet::new();

    // Write the manifest file
    let mut manifest_content =
        format!("Manifest-Version: 1.0\nMain-Class: {MAIN_CLASS_MANIFEST}\n");

    if !shade_libraries {
        let class_path = library_files
            .iter()
            .map(|n| {
                let relative_path = file
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(n)
                    .to_string_lossy()
                    .replace("\\", "/");
                relative_path
            })
            .collect::<Vec<_>>()
            .join(" ");
        manifest_content.push_str(&format!("Class-Path: {}\n", class_path));
    }

    zip_writer.start_file(MANIFEST_PATH, FileOptions::<()>::default())?;
    zip_writer
        .write_all(manifest_content.as_bytes())
        .map_err(|n| FabricInstallError::ZipEntryWriteError(n, MANIFEST_PATH.to_owned()))?;
    added_entries.insert(MANIFEST_PATH.to_string());

    // Write the fabric server launch properties
    let launch_properties = format!("launch.mainClass={}\n", launch_main_class);
    let launch_properties_path = "fabric-server-launch.properties";
    zip_writer.start_file(launch_properties_path, FileOptions::<()>::default())?;
    zip_writer
        .write_all(launch_properties.as_bytes())
        .map_err(|n| {
            FabricInstallError::ZipEntryWriteError(n, launch_properties_path.to_owned())
        })?;
    added_entries.insert("fabric-server-launch.properties".to_string());

    // Shade libraries if required
    if shade_libraries {
        let mut services = HashMap::<String, HashSet<String>>::new();

        for library_path in library_files {
            let library_file = File::open(library_path).map_err(io_err!(library_path))?;
            let mut jar_reader = zip::read::ZipArchive::new(BufReader::new(library_file))?;

            for i in 0..jar_reader.len() {
                let mut entry = jar_reader.by_index(i)?;
                let name = entry.name().to_string();

                if entry.is_dir() {
                    continue;
                }

                if name.starts_with(SERVICES_DIR) && name[SERVICES_DIR.len()..].find('/').is_none()
                {
                    // Parse and merge service definitions
                    let mut data = String::new();
                    entry
                        .read_to_string(&mut data)
                        .map_err(|n| FabricInstallError::ZipEntryReadError(n, name.clone()))?;
                    parse_service_definition(&name, &data, &mut services);
                } else if regex::Regex::new(r"META-INF/[^/]+\.(SF|DSA|RSA|EC)")
                    .unwrap()
                    .is_match(&name)
                {
                    // Ignore signature files
                } else if !added_entries.insert(name.clone()) {
                    // Duplicate entry, ignore
                } else {
                    // Write the entry to the output jar
                    zip_writer.start_file(&name, FileOptions::<()>::default())?;
                    io::copy(&mut entry, &mut zip_writer)
                        .map_err(|err| FabricInstallError::ZipEntryWriteError(err, name.clone()))?;
                }
            }
        }

        // Write the merged service definitions
        for (service_name, definitions) in services {
            zip_writer.start_file(&service_name, FileOptions::<()>::default())?;
            for definition in &definitions {
                writeln!(zip_writer, "{}", definition).map_err(|err| {
                    FabricInstallError::ZipEntryWriteError(err, service_name.clone())
                })?;
            }
        }
    }

    zip_writer.finish()?;
    Ok(())
}

fn parse_service_definition(
    name: &str,
    data: &str,
    services: &mut HashMap<String, HashSet<String>>,
) {
    for line in data.lines() {
        let trimmed_line = line.split('#').next().unwrap_or("").trim();

        if !trimmed_line.is_empty() {
            services
                .entry(name.to_string())
                .or_insert_with(HashSet::new)
                .insert(trimmed_line.to_string());
        }
    }
}
