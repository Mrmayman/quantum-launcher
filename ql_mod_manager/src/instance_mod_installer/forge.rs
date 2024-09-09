use std::{
    io::{Read, Seek},
    num::ParseIntError,
    path::{Path, PathBuf},
    process::Command,
    string::FromUtf8Error,
};

use ql_instances::{
    error::IoError,
    file_utils::{self, RequestError},
    io_err,
    java_install::{self, JavaInstallError},
    json_structs::{json_java_list::JavaVersion, json_version::VersionDetails, JsonDownloadError},
};

use crate::instance_mod_installer::forge_json::{JsonForgeDetails, JsonForgeVersions};

use super::forge_json::JsonForgeDetailsLibrary;

pub async fn install(instance_name: &str) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let lock_path = instance_dir.join("forge.lock");
    std::fs::write(
        &lock_path,
        "If you see this, forge was not installed correctly.",
    )
    .map_err(io_err!(lock_path))?;

    let version_json_path = instance_dir.join("details.json");
    let version_json =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json = serde_json::from_str::<VersionDetails>(&version_json)?;

    println!("[info] Installing forge: Downloading JSON");
    let minecraft_version = version_json.id;
    let forge_versions_json = JsonForgeVersions::download().await?;
    let forge_version = forge_versions_json
        .get_forge_version(&minecraft_version)
        .ok_or(ForgeInstallError::NoForgeVersionFound)?;

    let (
        short_forge_version,
        forge_major_version,
        forge_dir,
        client,
        installer_file,
        installer_name,
        installer_path,
    ) = download_forge_installer(minecraft_version, forge_version, instance_dir).await?;

    let (libraries_dir, classpath) = get_initial_classpath(
        &short_forge_version,
        &forge_dir,
        forge_major_version,
        &installer_name,
        installer_path,
    )
    .await?;

    let temp_dir = extract_zip_file(&installer_file)?;
    let forge_json_path = temp_dir.path().join("version.json");
    let forge_json = std::fs::read_to_string(&forge_json_path).map_err(io_err!(forge_json_path))?;
    let forge_json: JsonForgeDetails = serde_json::from_str(&forge_json)?;

    for library in forge_json
        .libraries
        .iter()
        .filter(|library| !matches!(library.clientreq, Some(false)))
    {
        let parts: Vec<&str> = library.name.split(':').collect();
        let class = parts[0];
        let lib = parts[1];
        let ver = parts[2];

        if class == "net.minecraftforge" && lib == "forge" {
            continue;
        }

        let (file, path) = get_filename_and_path(lib, ver, library, class)?;

        let url = if let Some(url) = &library.downloads.artifact.url {
            url.to_owned()
        } else {
            let baseurl = if let Some(url) = &library.url {
                url
            } else {
                "https://libraries.minecraft.net/"
            };
            format!("{baseurl}{path}/{file}")
        };

        let lib_dir_path = libraries_dir.join(&path);
        std::fs::create_dir_all(&lib_dir_path).map_err(io_err!(lib_dir_path))?;

        let dest = lib_dir_path.join(&file);
        let dest_str = dest
            .to_str()
            .ok_or(ForgeInstallError::PathBufToStr(dest.to_owned()))?;

        match file_utils::download_file_to_bytes(&client, &url).await {
            Ok(bytes) => {
                std::fs::write(&dest, &bytes).map_err(io_err!(dest))?;
            }
            Err(_) => {
                unpack_augmented_library(&client, dest_str, &url).await?;
            }
        };
    }

    todo!()
}

async fn unpack_augmented_library(
    client: &reqwest::Client,
    dest_str: &str,
    url: &str,
) -> Result<(), ForgeInstallError> {
    let bytes = file_utils::download_file_to_bytes(client, &format!("{url}.pack.xz")).await?;
    let temp_extract_xz = extract_zip_file(&bytes)?;
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
    let (file, path) = if let Some(full_path) = &library.downloads.artifact.path {
        let parent = PathBuf::from(full_path)
            .parent()
            .ok_or(ForgeInstallError::LibraryParentError)?
            .to_owned();
        (
            PathBuf::from(full_path)
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

async fn get_initial_classpath(
    short_forge_version: &str,
    forge_dir: &Path,
    forge_major_version: usize,
    installer_name: &str,
    installer_path: PathBuf,
) -> Result<(PathBuf, String), ForgeInstallError> {
    let libraries_dir = forge_dir.join("libraries");
    std::fs::create_dir_all(&libraries_dir).map_err(io_err!(libraries_dir))?;
    let classpath = if forge_major_version >= 27 {
        println!("[info] Installing forge: Getting Java Compiler");
        let javac_path = java_install::get_java_binary(JavaVersion::Java8, "javac", None).await?;
        let java_source_file = include_str!("../../../assets/ClientInstaller.java");

        let source_path = forge_dir.join("ClientInstaller.java");
        std::fs::write(&source_path, java_source_file).map_err(io_err!(source_path))?;

        let launcher_profiles_json_path = forge_dir.join("launcher_profiles.json");
        std::fs::write(&launcher_profiles_json_path, "{}")
            .map_err(io_err!(launcher_profiles_json_path))?;
        let launcher_profiles_json_microsoft_store_path =
            forge_dir.join("launcher_profiles_microsoft_store.json");
        std::fs::write(&launcher_profiles_json_microsoft_store_path, "{}")
            .map_err(io_err!(launcher_profiles_json_microsoft_store_path))?;

        println!("[info] Installing forge: Compiling Installer");
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
        let java_path = java_install::get_java_binary(JavaVersion::Java8, "java", None).await?;

        println!("[info] Installing Forge: Running Installer");
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
            format!("libraries/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}.jar:")
        } else {
            String::new()
        }
    } else {
        format!(
            "{}:",
            installer_path
                .to_str()
                .ok_or(ForgeInstallError::PathBufToStr(installer_path.to_owned()))?
        )
    };
    Ok((libraries_dir, classpath))
}

async fn download_forge_installer(
    minecraft_version: String,
    forge_version: String,
    instance_dir: PathBuf,
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
        .unwrap_or(&forge_version)
        .parse()?;
    let forge_dir = instance_dir.join("forge");
    std::fs::create_dir_all(&forge_dir).map_err(io_err!(forge_dir))?;
    let client = reqwest::Client::new();
    let file_type = if forge_major_version < 27 {
        "universal"
    } else {
        "installer"
    };
    println!("[info] Installing forge: Downloading Installer");
    let url = format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{short_forge_version}/forge-{short_forge_version}-{file_type}.jar");
    let installer_file = match file_utils::download_file_to_bytes(&client, &url).await {
        Ok(file) => file,
        Err(_) => {
            let url = format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{norm_forge_version}/forge-{norm_forge_version}-{file_type}.jar");
            file_utils::download_file_to_bytes(&client, &url).await?
        }
    };
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

#[derive(Debug)]
pub enum ForgeInstallError {
    Io(IoError),
    Request(RequestError),
    Serde(serde_json::Error),
    NoForgeVersionFound,
    ParseIntError(ParseIntError),
    TempFile(std::io::Error),
    JavaInstallError(JavaInstallError),
    PathBufToStr(PathBuf),
    CompileError(String, String),
    InstallerError(String, String),
    Unpack200Error(String, String),
    FromUtf8Error(FromUtf8Error),
    LibraryParentError,
}

impl From<IoError> for ForgeInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<RequestError> for ForgeInstallError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<serde_json::Error> for ForgeInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<ParseIntError> for ForgeInstallError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl From<JavaInstallError> for ForgeInstallError {
    fn from(value: JavaInstallError) -> Self {
        Self::JavaInstallError(value)
    }
}

impl From<FromUtf8Error> for ForgeInstallError {
    fn from(value: FromUtf8Error) -> Self {
        Self::FromUtf8Error(value)
    }
}

impl From<JsonDownloadError> for ForgeInstallError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => Self::Request(err),
            JsonDownloadError::SerdeError(err) => Self::Serde(err),
        }
    }
}
