use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};

use error::ForgeInstallError;
use ql_instances::{
    file_utils::{self, RequestError},
    info, io_err,
    java_install::{self},
    json_structs::{
        json_forge::{
            JsonForgeDetails, JsonForgeDetailsLibrary, JsonForgeInstallProfile, JsonForgeVersions,
        },
        json_java_list::JavaVersion,
        json_version::VersionDetails,
    },
    JavaInstallProgress,
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

use crate::instance_mod_installer::change_instance_type;

mod error;
mod uninstall;

pub use uninstall::{uninstall, uninstall_wrapped};

pub async fn install_wrapped(
    instance_name: String,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<JavaInstallProgress>>,
) -> Result<(), String> {
    install(&instance_name, f_progress, j_progress)
        .await
        .map_err(|err| err.to_string())
}

pub enum ForgeInstallProgress {
    P1Start,
    P2DownloadingJson,
    P3DownloadingInstaller,
    P4RunningInstaller,
    P5DownloadingLibrary { num: usize, out_of: usize },
    P6Done,
}

pub async fn install(
    instance_name: &str,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<JavaInstallProgress>>,
) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let mods_dir_path = instance_dir.join(".minecraft/mods");
    std::fs::create_dir_all(&mods_dir_path).map_err(io_err!(mods_dir_path))?;

    let lock_path = instance_dir.join("forge.lock");
    info!("Started installing forge");

    if let Some(progress) = &f_progress {
        progress.send(ForgeInstallProgress::P1Start).unwrap();
    }

    std::fs::write(
        &lock_path,
        "If you see this, forge was not installed correctly.",
    )
    .map_err(io_err!(lock_path))?;

    let version_json_path = instance_dir.join("details.json");
    let version_json =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json = serde_json::from_str::<VersionDetails>(&version_json)?;

    let minecraft_version = version_json.id;
    println!("- Downloading JSON");
    if let Some(progress) = &f_progress {
        progress
            .send(ForgeInstallProgress::P2DownloadingJson)
            .unwrap();
    }
    let forge_versions_json = JsonForgeVersions::download().await?;
    let forge_version = forge_versions_json
        .get_forge_version(&minecraft_version)
        .ok_or(ForgeInstallError::NoForgeVersionFound)?;
    info!("Forge version {forge_version} is being installed");

    let (
        short_forge_version,
        forge_major_version,
        forge_dir,
        client,
        installer_file,
        installer_name,
        installer_path,
    ) = download_forge_installer(
        &minecraft_version,
        &forge_version,
        &instance_dir,
        f_progress.as_ref(),
    )
    .await?;

    let (libraries_dir, mut classpath) = run_installer_and_get_classpath(
        &short_forge_version,
        &forge_dir,
        forge_major_version,
        &installer_name,
        installer_path,
        f_progress.as_ref(),
        j_progress,
    )
    .await?;

    let forge_json = get_forge_json(installer_file)?;

    let num_libraries = forge_json
        .libraries
        .iter()
        .filter(|library| !matches!(library.clientreq, Some(false)))
        .count();

    for (library_i, library) in forge_json
        .libraries
        .iter()
        .filter(|library| !matches!(library.clientreq, Some(false)))
        .enumerate()
    {
        if !download_library(
            library,
            library_i,
            num_libraries,
            &client,
            &libraries_dir,
            &mut classpath,
            f_progress.as_ref(),
        )
        .await?
        {
            continue;
        }
    }

    let classpath_path = forge_dir.join("classpath.txt");
    std::fs::write(&classpath_path, &classpath).map_err(io_err!(classpath_path))?;

    let json_path = forge_dir.join("details.json");
    std::fs::write(&json_path, serde_json::to_string(&forge_json)?).map_err(io_err!(json_path))?;

    change_instance_type(&instance_dir, "Forge".to_owned())?;

    std::fs::remove_file(&lock_path).map_err(io_err!(lock_path))?;
    Ok(())
}

fn get_forge_json(installer_file: Vec<u8>) -> Result<JsonForgeDetails, ForgeInstallError> {
    let temp_dir = extract_zip_file(&installer_file)?;
    let forge_json_path = temp_dir.path().join("version.json");
    if forge_json_path.exists() {
        let forge_json =
            std::fs::read_to_string(&forge_json_path).map_err(io_err!(forge_json_path))?;

        let forge_json: JsonForgeDetails = serde_json::from_str(&forge_json)?;
        Ok(forge_json)
    } else {
        let forge_json_path = temp_dir.path().join("install_profile.json");
        if forge_json_path.exists() {
            let forge_json =
                std::fs::read_to_string(&forge_json_path).map_err(io_err!(forge_json_path))?;

            let forge_json: JsonForgeInstallProfile = serde_json::from_str(&forge_json)?;
            Ok(forge_json.versionInfo)
        } else {
            Err(ForgeInstallError::NoInstallJson)
        }
    }
}

async fn download_library(
    library: &JsonForgeDetailsLibrary,
    library_i: usize,
    num_libraries: usize,
    client: &reqwest::Client,
    libraries_dir: &Path,
    classpath: &mut String,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
) -> Result<bool, ForgeInstallError> {
    let parts: Vec<&str> = library.name.split(':').collect();
    let class = parts[0];
    let lib = parts[1];
    let ver = parts[2];

    if class == "net.minecraftforge" && lib == "forge" {
        return Ok(false);
    }

    let (file, path) = get_filename_and_path(lib, ver, library, class)?;

    let url = if let Some(downloads) = &library.downloads {
        downloads.artifact.url.to_owned()
    } else {
        let baseurl = if let Some(url) = &library.url {
            url.to_owned()
        } else {
            "https://libraries.minecraft.net/".to_owned()
        };
        format!("{baseurl}{path}/{file}")
    };

    let lib_dir_path = libraries_dir.join(&path);
    std::fs::create_dir_all(&lib_dir_path).map_err(io_err!(lib_dir_path))?;

    let dest = lib_dir_path.join(&file);
    let dest_str = dest
        .to_str()
        .ok_or(ForgeInstallError::PathBufToStr(dest.to_owned()))?;

    info!(
        "Installing forge: Downloading library ({}/{num_libraries}): {}",
        library_i + 1,
        library.name
    );

    if let Some(progress) = &f_progress {
        progress
            .send(ForgeInstallProgress::P5DownloadingLibrary {
                num: library_i + 1,
                out_of: num_libraries,
            })
            .unwrap();
    }

    if dest.exists() {
        info!("Library already exists.");
    } else {
        match file_utils::download_file_to_bytes(client, &url).await {
            Ok(bytes) => {
                std::fs::write(&dest, bytes).map_err(io_err!(dest))?;
            }
            Err(err) => {
                eprintln!(
                    "[error] Error downloading library: {err}\n        Trying pack.xz version"
                );
                let result = unpack_augmented_library(client, dest_str, &url).await;
                if result.is_not_found() {
                    eprintln!("[error] Error 404 not found. Skipping...");
                    return Ok(true);
                } else {
                    result?;
                }
            }
        };
    }

    let classpath_item = libraries_dir.join(format!("{path}/{file}{CLASSPATH_SEPARATOR}"));

    classpath.push_str(
        classpath_item
            .to_str()
            .ok_or(ForgeInstallError::PathBufToStr(classpath_item.to_owned()))?,
    );

    Ok(true)
}

async fn unpack_augmented_library(
    client: &reqwest::Client,
    dest_str: &str,
    url: &str,
) -> Result<(), ForgeInstallError> {
    println!("- Unpacking augmented library");
    println!("- Downloading File");
    let bytes = file_utils::download_file_to_bytes(client, &format!("{url}.pack.xz")).await?;
    println!("- Extracting pack.xz");
    let temp_extract_xz = extract_zip_file(&bytes)?;

    println!("- Reading signature");
    let extracted_pack_path = temp_extract_xz.path().join(format!("{dest_str}.pack"));
    let mut extracted_pack =
        std::fs::File::open(&extracted_pack_path).map_err(io_err!(extracted_pack_path))?;
    extracted_pack
        .seek(std::io::SeekFrom::End(-8))
        .map_err(io_err!(extracted_pack_path))?;
    let mut sig_len_bytes = [0u8; 4];
    extracted_pack
        .read_exact(&mut sig_len_bytes)
        .map_err(io_err!(extracted_pack_path))?;
    let sig_len = u32::from_le_bytes(sig_len_bytes);

    let full_len = std::fs::metadata(&extracted_pack_path)
        .map_err(io_err!(extracted_pack_path))?
        .len() as usize;
    let crop_len = full_len - sig_len as usize - 8;

    let extracted_pack =
        std::fs::File::open(&extracted_pack_path).map_err(io_err!(extracted_pack_path))?;
    let mut pack_crop = Vec::with_capacity(crop_len);
    extracted_pack
        .take(crop_len as u64)
        .read_to_end(&mut pack_crop)
        .map_err(io_err!(extracted_pack_path))?;

    let cropped_pack_path = temp_extract_xz
        .path()
        .join(format!("{dest_str}.pack.crop",));
    std::fs::write(&cropped_pack_path, &pack_crop).map_err(io_err!(cropped_pack_path))?;

    println!("- Unpacking extracted file");
    let unpack200_path =
        java_install::get_java_binary(JavaVersion::Java8, "unpack200", None).await?;
    let output = Command::new(&unpack200_path)
        .args(&[format!("{dest_str}.pack.crop",), dest_str.to_owned()])
        .output()
        .map_err(io_err!(unpack200_path))?;

    if !output.status.success() {
        return Err(ForgeInstallError::Unpack200Error(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }
    Ok(())
}

fn get_filename_and_path(
    lib: &str,
    ver: &str,
    library: &JsonForgeDetailsLibrary,
    class: &str,
) -> Result<(String, String), ForgeInstallError> {
    let (file, path) = if let Some(downloads) = &library.downloads {
        let parent = PathBuf::from(&downloads.artifact.path)
            .parent()
            .ok_or(ForgeInstallError::LibraryParentError)?
            .to_owned();
        (
            PathBuf::from(&downloads.artifact.path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            parent
                .to_str()
                .ok_or(ForgeInstallError::PathBufToStr(parent.to_owned()))?
                .to_owned(),
        )
    } else {
        (
            format!("{lib}-{ver}.jar"),
            format!("{}/{lib}/{ver}", class.replace('.', "/")),
        )
    };
    Ok((file, path))
}

async fn run_installer_and_get_classpath(
    short_forge_version: &str,
    forge_dir: &Path,
    forge_major_version: usize,
    installer_name: &str,
    installer_path: PathBuf,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<JavaInstallProgress>>,
) -> Result<(PathBuf, String), ForgeInstallError> {
    let libraries_dir = forge_dir.join("libraries");
    std::fs::create_dir_all(&libraries_dir).map_err(io_err!(libraries_dir))?;

    let classpath = if forge_major_version >= 14 {
        let javac_path =
            java_install::get_java_binary(JavaVersion::Java21, "javac", j_progress).await?;
        let java_source_file = include_str!("../../../../assets/ClientInstaller.java");

        let source_path = forge_dir.join("ClientInstaller.java");
        std::fs::write(&source_path, java_source_file).map_err(io_err!(source_path))?;

        let launcher_profiles_json_path = forge_dir.join("launcher_profiles.json");
        std::fs::write(&launcher_profiles_json_path, "{}")
            .map_err(io_err!(launcher_profiles_json_path))?;
        let launcher_profiles_json_microsoft_store_path =
            forge_dir.join("launcher_profiles_microsoft_store.json");
        std::fs::write(&launcher_profiles_json_microsoft_store_path, "{}")
            .map_err(io_err!(launcher_profiles_json_microsoft_store_path))?;

        println!("- Compiling Installer");
        if let Some(progress) = &f_progress {
            progress
                .send(ForgeInstallProgress::P4RunningInstaller)
                .unwrap();
        }
        let output = Command::new(&javac_path)
            .args(["-cp", &installer_name, "ClientInstaller.java", "-d", "."])
            .current_dir(forge_dir)
            .output()
            .map_err(io_err!(javac_path))?;

        if !output.status.success() {
            return Err(ForgeInstallError::CompileError(
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
            ));
        }
        let java_path = java_install::get_java_binary(JavaVersion::Java21, "java", None).await?;

        println!("- Running Installer");
        let output = Command::new(&java_path)
            .args(["-cp", &format!("{}:.", installer_name), "ClientInstaller"])
            .current_dir(forge_dir)
            .output()
            // .spawn()
            .map_err(io_err!(java_path))?;

        if !output.status.success() {
            return Err(ForgeInstallError::InstallerError(
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
            ));
        }

        if forge_major_version < 39 {
            format!("{}/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}.jar{CLASSPATH_SEPARATOR}", libraries_dir.to_str().ok_or(ForgeInstallError::PathBufToStr(libraries_dir.to_owned()))?)
        } else {
            String::new()
        }
    } else {
        format!(
            "{}{CLASSPATH_SEPARATOR}",
            installer_path
                .to_str()
                .ok_or(ForgeInstallError::PathBufToStr(installer_path.to_owned()))?
        )
    };
    Ok((libraries_dir, classpath))
}

async fn download_forge_installer(
    minecraft_version: &str,
    forge_version: &str,
    instance_dir: &Path,
    progress: Option<&Sender<ForgeInstallProgress>>,
) -> Result<
    (
        String,
        usize,
        PathBuf,
        reqwest::Client,
        Vec<u8>,
        String,
        PathBuf,
    ),
    ForgeInstallError,
> {
    let norm_version = {
        let number_of_full_stops = minecraft_version.chars().filter(|c| *c == '.').count();
        if number_of_full_stops == 1 {
            format!("{minecraft_version}.0")
        } else {
            minecraft_version.to_owned()
        }
    };
    let short_forge_version = format!("{minecraft_version}-{forge_version}");
    let norm_forge_version = format!("{short_forge_version}-{norm_version}");
    let forge_major_version: usize = forge_version
        .split('.')
        .next()
        .unwrap_or(forge_version)
        .parse()?;

    let forge_dir = instance_dir.join("forge");
    std::fs::create_dir_all(&forge_dir).map_err(io_err!(forge_dir))?;

    let client = reqwest::Client::new();

    let file_type = if forge_major_version < 14 {
        "universal"
    } else {
        "installer"
    };

    let file_type_flipped = if forge_major_version < 14 {
        "installer"
    } else {
        "universal"
    };

    println!("- Downloading Installer");
    if let Some(progress) = &progress {
        progress
            .send(ForgeInstallProgress::P3DownloadingInstaller)
            .unwrap();
    }

    let installer_file = try_downloading_from_urls(&client, &[
        &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}-{file_type}.jar"),
        &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{norm_forge_version}/forge-{norm_forge_version}-{file_type}.jar"),
        &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}-{file_type_flipped}.jar"),
        &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{norm_forge_version}/forge-{norm_forge_version}-{file_type_flipped}.jar"),
    ]).await?;

    let installer_name = format!("forge-{short_forge_version}-{file_type}.jar");
    let installer_path = forge_dir.join(&installer_name);
    std::fs::write(&installer_path, &installer_file).map_err(io_err!(installer_path))?;
    Ok((
        short_forge_version,
        forge_major_version,
        forge_dir,
        client,
        installer_file,
        installer_name,
        installer_path,
    ))
}

pub fn extract_zip_file(archive: &[u8]) -> Result<tempfile::TempDir, ForgeInstallError> {
    // Create a temporary directory
    let temp_dir = match tempfile::TempDir::new() {
        Ok(temp_dir) => temp_dir,
        Err(err) => return Err(ForgeInstallError::TempFile(err)),
    };

    let target_dir = std::path::PathBuf::from(temp_dir.path());

    // The third parameter allows you to strip away toplevel directories.
    // If `archive` contained a single folder, that folder's contents would be extracted instead.
    zip_extract::extract(std::io::Cursor::new(archive), &target_dir, true)
        .expect("Could not extract .sb3 zip");

    Ok(temp_dir)
}

trait Is404NotFound {
    fn is_not_found(&self) -> bool;
}

impl<T> Is404NotFound for Result<T, ForgeInstallError> {
    fn is_not_found(&self) -> bool {
        if let Err(ForgeInstallError::Request(RequestError::DownloadError { code, .. })) = &self {
            code.as_u16() == 404
        } else {
            false
        }
    }
}

impl Is404NotFound for RequestError {
    fn is_not_found(&self) -> bool {
        if let RequestError::DownloadError { code, .. } = &self {
            code.as_u16() == 404
        } else {
            false
        }
    }
}

async fn try_downloading_from_urls(
    client: &reqwest::Client,
    urls: &[&str],
) -> Result<Vec<u8>, ForgeInstallError> {
    let num_urls = urls.len();
    for (i, url) in urls.iter().enumerate() {
        let result = file_utils::download_file_to_bytes(client, url).await;

        match result {
            Ok(file) => return Ok(file),
            Err(err) => {
                let is_last_url = i + 1 == num_urls;
                if err.is_not_found() && !is_last_url {
                    continue;
                } else {
                    Err(err)?
                }
            }
        }
    }
    unreachable!()
}
