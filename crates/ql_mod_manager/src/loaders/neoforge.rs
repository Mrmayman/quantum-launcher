use std::{path::Path, sync::mpsc::Sender};

use chrono::DateTime;
use ql_core::{
    file_utils, info, json::VersionDetails, pt, GenericProgress, InstanceSelection, IntoIoError,
    IoError, CLASSPATH_SEPARATOR,
};
use ql_java_handler::{get_java_binary, JavaVersion};
use serde::Deserialize;
use tokio::process::Command;

use crate::loaders::change_instance_type;

use super::forge::{ForgeInstallError, ForgeInstallProgress};

const NEOFORGE_VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

const INSTALLER_NAME: &str = "installer.jar";

#[derive(Deserialize)]
struct NeoforgeVersions {
    versions: Vec<String>,
}

pub async fn install(
    instance: InstanceSelection,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<GenericProgress>>,
) -> Result<(), ForgeInstallError> {
    info!("Installing NeoForge");
    let neoforge_version = get_neoforge_version(f_progress.as_ref(), &instance).await?;

    send_progress(
        f_progress.as_ref(),
        ForgeInstallProgress::P3DownloadingInstaller,
    );
    let installer_url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar");
    let installer_bytes = file_utils::download_file_to_bytes(&installer_url, false).await?;

    let instance_dir = file_utils::get_instance_dir(&instance).await?;
    let neoforge_dir = instance_dir.join("forge");
    tokio::fs::create_dir_all(&neoforge_dir)
        .await
        .path(&neoforge_dir)?;
    if !instance.is_server() {
        create_required_jsons(&neoforge_dir).await?;
    }

    let installer_path = neoforge_dir.join(INSTALLER_NAME);
    tokio::fs::write(&installer_path, &installer_bytes)
        .await
        .path(&installer_path)?;

    compile_and_run_installer(
        &neoforge_dir,
        j_progress.as_ref(),
        f_progress.as_ref(),
        instance.is_server(),
    )
    .await?;

    delete(&neoforge_dir, "ForgeInstaller.java").await?;
    delete(&neoforge_dir, "ForgeInstaller.class").await?;
    delete(&neoforge_dir, "launcher_profiles.json").await?;
    delete(&neoforge_dir, "launcher_profiles_microsoft_store.json").await?;

    if !instance.is_server() {
        let jar_version_json = get_version_json(&installer_bytes, &neoforge_dir).await?;

        let libraries_path = neoforge_dir.join("libraries");
        tokio::fs::create_dir_all(&libraries_path)
            .await
            .path(&libraries_path)?;

        let mut classpath = String::new();
        let mut clean_classpath = String::new();

        let len = jar_version_json.libraries.len();
        for (i, library) in jar_version_json
            .libraries
            .iter()
            .filter(|n| n.clientreq.unwrap_or(true))
            .enumerate()
        {
            info!("Downloading library {i}/{len}: {}", library.name);
            send_progress(
                f_progress.as_ref(),
                ForgeInstallProgress::P5DownloadingLibrary {
                    num: i,
                    out_of: len,
                },
            );
            let parts: Vec<&str> = library.name.split(':').collect();

            let class = parts[0];
            let lib = parts[1];
            let ver = parts[2];

            clean_classpath.push_str(&format!("{class}:{lib}\n"));

            let (url, path) = if let Some(downloads) = &library.downloads {
                (
                    downloads.artifact.url.clone(),
                    downloads.artifact.path.clone(),
                )
            } else {
                let base = library
                    .url
                    .clone()
                    .unwrap_or("https://libraries.minecraft.net/".to_owned());
                let path = format!("{}/{lib}/{ver}/{lib}-{ver}.jar", class.replace(".", "/"));

                let url = base + &path;
                (url, path)
            };

            classpath.push_str(&format!("../forge/libraries/{path}"));
            classpath.push(CLASSPATH_SEPARATOR);

            let file_path = libraries_path.join(&path);
            if file_path.exists() {
                continue;
            }

            let dir_path = file_path.parent().unwrap();
            tokio::fs::create_dir_all(dir_path).await.path(dir_path)?;

            // WTF: I am NOT dealing with the unpack200 augmented library NONSENSE
            // because I haven't seen the launcher using it ONCE.
            // Please open an issue if you actually need it.
            let file_bytes = file_utils::download_file_to_bytes(&url, false).await?;
            tokio::fs::write(&file_path, &file_bytes)
                .await
                .path(&file_path)?;
        }

        let classpath_path = neoforge_dir.join("classpath.txt");
        tokio::fs::write(&classpath_path, &classpath)
            .await
            .path(&classpath_path)?;

        let classpath_path = neoforge_dir.join("clean_classpath.txt");
        tokio::fs::write(&classpath_path, &clean_classpath)
            .await
            .path(&classpath_path)?;
    }

    info!("Finished installing NeoForge");

    change_instance_type(&instance_dir, "NeoForge".to_owned()).await?;

    Ok(())
}

async fn get_version_json(
    installer_bytes: &[u8],
    neoforge_dir: &Path,
) -> Result<ql_core::json::forge::JsonDetails, ForgeInstallError> {
    let temp_extract = extract_zip_file(installer_bytes)?;
    let jar_version_path = temp_extract.path().join("version.json");
    let jar_version_json = tokio::fs::read_to_string(&jar_version_path)
        .await
        .path(&jar_version_path)?;
    let out_jar_version_path = neoforge_dir.join("details.json");
    tokio::fs::write(&out_jar_version_path, &jar_version_json)
        .await
        .path(&out_jar_version_path)?;
    let jar_version_json: ql_core::json::forge::JsonDetails =
        serde_json::from_str(&jar_version_json)?;
    Ok(jar_version_json)
}

fn send_progress(f_progress: Option<&Sender<ForgeInstallProgress>>, message: ForgeInstallProgress) {
    if let Some(progress) = f_progress {
        _ = progress.send(message);
    }
}

async fn get_neoforge_version(
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    instance_selection: &InstanceSelection,
) -> Result<String, ForgeInstallError> {
    pt!("Checking NeoForge versions");
    send_progress(f_progress, ForgeInstallProgress::P2DownloadingJson);
    let versions = file_utils::download_file_to_string(NEOFORGE_VERSIONS_URL, false).await?;
    let versions: NeoforgeVersions = serde_json::from_str(&versions)?;

    let version_json = VersionDetails::load(instance_selection).await?;
    let release_time = DateTime::parse_from_rfc3339(&version_json.releaseTime)?;

    let v1_20_2 = DateTime::parse_from_rfc3339("2023-09-20T09:02:57+00:00")?;
    if release_time < v1_20_2 {
        return Err(ForgeInstallError::OutdatedMinecraft);
    }

    let mut start_pattern = version_json.id[2..].to_owned();
    if !start_pattern.contains('.') {
        // "20" (in 1.20) -> "20.0" (in 1.20.0)
        // Ensures there are a constant number of parts in the version.
        start_pattern.push_str(".0");
    }

    let neoforge_version = versions
        .versions
        .iter()
        .filter(|n| n.starts_with(&start_pattern))
        .next_back()
        .ok_or(ForgeInstallError::NoForgeVersionFound)?
        .clone();

    Ok(neoforge_version)
}

async fn delete(dir: &Path, path: &str) -> Result<(), IoError> {
    let delete_path = dir.join(path);
    if delete_path.exists() {
        tokio::fs::remove_file(&delete_path)
            .await
            .path(delete_path)?;
    }
    Ok(())
}

async fn compile_and_run_installer(
    neoforge_dir: &Path,
    j_progress: Option<&Sender<GenericProgress>>,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    is_server: bool,
) -> Result<(), ForgeInstallError> {
    send_progress(f_progress, ForgeInstallProgress::P4RunningInstaller);
    let javac_path = get_java_binary(JavaVersion::Java21, "javac", j_progress).await?;
    let java_source_file = include_str!("../../../../assets/installers/ForgeInstaller.java")
        .replace("CLIENT", if is_server { "SERVER" } else { "CLIENT" })
        .replace("new File(\".\")", "new File(\".\"), a -> true");
    let source_path = neoforge_dir.join("ForgeInstaller.java");
    tokio::fs::write(&source_path, java_source_file)
        .await
        .path(source_path)?;

    pt!("Compiling Installer");
    let output = Command::new(&javac_path)
        .args(["-cp", INSTALLER_NAME, "ForgeInstaller.java", "-d", "."])
        .current_dir(neoforge_dir)
        .output()
        .await
        .path(javac_path)?;
    if !output.status.success() {
        return Err(ForgeInstallError::CompileError(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }

    let java_path = get_java_binary(JavaVersion::Java21, "java", None).await?;
    pt!("Running Installer");
    let output = Command::new(&java_path)
        .args([
            "-cp",
            &format!("{INSTALLER_NAME}{CLASSPATH_SEPARATOR}."),
            "ForgeInstaller",
        ])
        .current_dir(neoforge_dir)
        .output()
        .await
        .path(java_path)?;

    if !output.status.success() {
        return Err(ForgeInstallError::InstallerError(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }

    Ok(())
}

async fn create_required_jsons(neoforge_dir: &Path) -> Result<(), ForgeInstallError> {
    const CONTENTS: &str = "{}";

    let launcher_profiles_json_path = neoforge_dir.join("launcher_profiles.json");
    tokio::fs::write(&launcher_profiles_json_path, "{}")
        .await
        .path(launcher_profiles_json_path)?;

    let launcher_profiles_json_microsoft_store_path =
        neoforge_dir.join("launcher_profiles_microsoft_store.json");
    tokio::fs::write(&launcher_profiles_json_microsoft_store_path, CONTENTS)
        .await
        .path(launcher_profiles_json_microsoft_store_path)?;

    Ok(())
}

pub fn extract_zip_file(archive: &[u8]) -> Result<tempfile::TempDir, ForgeInstallError> {
    let temp_dir = match tempfile::TempDir::new() {
        Ok(temp_dir) => temp_dir,
        Err(err) => return Err(ForgeInstallError::TempFile(err)),
    };

    let target_dir = std::path::PathBuf::from(temp_dir.path());

    zip_extract::extract(std::io::Cursor::new(archive), &target_dir, true)?;

    Ok(temp_dir)
}
