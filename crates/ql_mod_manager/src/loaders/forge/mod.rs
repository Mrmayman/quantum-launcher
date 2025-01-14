use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};

use error::{ForgeInstallError, Is404NotFound};
use ql_core::{
    err, file_utils, get_java_binary, info,
    json::{
        forge::{
            JsonForgeDetails, JsonForgeDetailsLibrary, JsonForgeInstallProfile, JsonForgeVersions,
        },
        java_list::JavaVersion,
        version::VersionDetails,
    },
    pt, GenericProgress, InstanceSelection, IntoIoError,
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

use crate::loaders::change_instance_type;

mod error;
mod server;
pub use server::{install_server, install_server_w};
mod uninstall;

pub use uninstall::{
    uninstall_client, uninstall_client_w, uninstall_server, uninstall_server_w, uninstall_w,
};

struct ForgeInstaller {
    f_progress: Option<Sender<ForgeInstallProgress>>,
    norm_forge_version: String,
    short_version: String,
    major_version: usize,
    instance_dir: PathBuf,
    forge_dir: PathBuf,
    client: reqwest::Client,
    is_server: bool,
}

impl ForgeInstaller {
    fn remove_lock(&self) -> Result<(), ForgeInstallError> {
        let lock_path = self.instance_dir.join("forge.lock");
        std::fs::remove_file(&lock_path).path(lock_path)?;
        Ok(())
    }

    async fn new(
        f_progress: Option<Sender<ForgeInstallProgress>>,
        instance_name: InstanceSelection,
    ) -> Result<Self, ForgeInstallError> {
        let client = reqwest::Client::new();

        let instance_dir = get_instance_dir(&instance_name)?;
        let forge_dir = if instance_name.is_server() {
            instance_dir.clone()
        } else {
            get_forge_dir(&instance_dir)?
        };

        let minecraft_version = get_minecraft_version(&instance_dir)?;

        create_mods_dir(&instance_dir)?;
        create_lock_file(&instance_dir)?;

        pt!("Downloading JSON");
        if let Some(progress) = &f_progress {
            progress
                .send(ForgeInstallProgress::P2DownloadingJson)
                .unwrap();
        }

        let version = get_forge_version(&minecraft_version).await?;

        info!("Forge version {version} is being installed");

        let norm_version = {
            let number_of_full_stops = minecraft_version.chars().filter(|c| *c == '.').count();
            if number_of_full_stops == 1 {
                format!("{minecraft_version}.0")
            } else {
                minecraft_version.clone()
            }
        };
        let short_version = format!("{minecraft_version}-{version}");
        let norm_forge_version = format!("{short_version}-{norm_version}");
        let major_version: usize = version.split('.').next().unwrap_or(&version).parse()?;

        Ok(Self {
            f_progress,
            norm_forge_version,
            short_version,
            major_version,
            instance_dir,
            forge_dir,
            client,
            is_server: instance_name.is_server(),
        })
    }

    async fn download_forge_installer(
        &self,
    ) -> Result<(Vec<u8>, String, PathBuf), ForgeInstallError> {
        let (file_type, file_type_flipped) = if self.major_version < 14 {
            ("universal", "installer")
        } else {
            ("installer", "universal")
        };

        pt!("Downloading Installer");
        self.send_progress(ForgeInstallProgress::P3DownloadingInstaller);

        let installer_file = self.try_downloading_from_urls(&[
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-{file_type}.jar", self.short_version, self.short_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-{file_type}.jar", self.norm_forge_version, self.norm_forge_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-{file_type_flipped}.jar", self.short_version, self.short_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-{file_type_flipped}.jar", self.norm_forge_version, self.norm_forge_version),
        ]).await?;

        let installer_name = format!("forge-{}-{file_type}.jar", self.short_version);
        let installer_path = self.forge_dir.join(&installer_name);
        std::fs::write(&installer_path, &installer_file).path(&installer_path)?;
        Ok((installer_file, installer_name, installer_path))
    }

    fn send_progress(&self, message: ForgeInstallProgress) {
        if let Some(progress) = &self.f_progress {
            progress.send(message).unwrap();
        }
    }

    async fn try_downloading_from_urls(&self, urls: &[&str]) -> Result<Vec<u8>, ForgeInstallError> {
        let num_urls = urls.len();
        for (i, url) in urls.iter().enumerate() {
            let result = file_utils::download_file_to_bytes(&self.client, url, false).await;

            match result {
                Ok(file) => return Ok(file),
                Err(err) => {
                    let is_last_url = i + 1 == num_urls;
                    if err.is_not_found() && !is_last_url {
                        continue;
                    }
                    return Err(ForgeInstallError::Request(err));
                }
            }
        }
        unreachable!()
    }

    async fn run_installer_and_get_classpath(
        &self,
        installer_name: &str,
        installer_path: PathBuf,
        j_progress: Option<Sender<GenericProgress>>,
    ) -> Result<(PathBuf, String), ForgeInstallError> {
        let libraries_dir = self.forge_dir.join("libraries");
        std::fs::create_dir_all(&libraries_dir).path(&libraries_dir)?;

        let classpath = if self.major_version >= 14 {
            self.run_installer(j_progress, installer_name).await?;

            if self.major_version < 39 {
                format!(
                    "{}/net/minecraftforge/forge/{}/forge-{}.jar{CLASSPATH_SEPARATOR}",
                    libraries_dir
                        .to_str()
                        .ok_or(ForgeInstallError::PathBufToStr(libraries_dir.clone()))?,
                    self.short_version,
                    self.short_version
                )
            } else {
                String::new()
            }
        } else {
            format!(
                "{}{CLASSPATH_SEPARATOR}",
                installer_path
                    .to_str()
                    .ok_or(ForgeInstallError::PathBufToStr(installer_path.clone()))?
            )
        };
        Ok((libraries_dir, classpath))
    }

    async fn run_installer(
        &self,
        j_progress: Option<Sender<GenericProgress>>,
        installer_name: &str,
    ) -> Result<(), ForgeInstallError> {
        let javac_path = get_java_binary(JavaVersion::Java21, "javac", j_progress).await?;
        let java_source_file = include_str!("../../../../../assets/installers/ForgeInstaller.java")
            .replace("CLIENT", if self.is_server { "SERVER" } else { "CLIENT" });
        let source_path = self.forge_dir.join("ClientInstaller.java");
        std::fs::write(&source_path, java_source_file).path(source_path)?;

        if !self.is_server {
            let launcher_profiles_json_path = self.forge_dir.join("launcher_profiles.json");
            std::fs::write(&launcher_profiles_json_path, "{}").path(launcher_profiles_json_path)?;
            let launcher_profiles_json_microsoft_store_path = self
                .forge_dir
                .join("launcher_profiles_microsoft_store.json");
            std::fs::write(&launcher_profiles_json_microsoft_store_path, "{}")
                .path(launcher_profiles_json_microsoft_store_path)?;
        }

        pt!("Compiling Installer");
        self.send_progress(ForgeInstallProgress::P4RunningInstaller);
        let output = Command::new(&javac_path)
            .args(["-cp", installer_name, "ClientInstaller.java", "-d", "."])
            .current_dir(&self.forge_dir)
            .output()
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
                &format!("{installer_name}{CLASSPATH_SEPARATOR}."),
                "ClientInstaller",
            ])
            .current_dir(&self.forge_dir)
            .output()
            // .spawn()
            .path(java_path)?;
        if !output.status.success() {
            return Err(ForgeInstallError::InstallerError(
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
            ));
        }
        Ok(())
    }

    fn get_forge_json(installer_file: &[u8]) -> Result<JsonForgeDetails, ForgeInstallError> {
        let temp_dir = Self::extract_zip_file(installer_file)?;
        let forge_json_path = temp_dir.path().join("version.json");
        if forge_json_path.exists() {
            let forge_json = std::fs::read_to_string(&forge_json_path).path(forge_json_path)?;

            let forge_json: JsonForgeDetails = serde_json::from_str(&forge_json)?;
            Ok(forge_json)
        } else {
            let forge_json_path = temp_dir.path().join("install_profile.json");
            if forge_json_path.exists() {
                let forge_json = std::fs::read_to_string(&forge_json_path).path(forge_json_path)?;

                let forge_json: JsonForgeInstallProfile = serde_json::from_str(&forge_json)?;
                Ok(forge_json.versionInfo)
            } else {
                Err(ForgeInstallError::NoInstallJson)
            }
        }
    }

    async fn download_library(
        &self,
        library: &JsonForgeDetailsLibrary,
        library_i: usize,
        num_libraries: usize,
        libraries_dir: &Path,
        classpath: &mut String,
        clean_classpath: &mut String,
    ) -> Result<(), ForgeInstallError> {
        let parts: Vec<&str> = library.name.split(':').collect();
        let class = parts[0];
        let lib = parts[1];
        let ver = parts[2];

        clean_classpath.push_str(&format!("{}:{}\n", parts[0], parts[1]));

        let (file, path) = Self::get_filename_and_path(lib, ver, library, class)?;

        if class == "net.minecraftforge" && lib == "forge" {
            if self.major_version > 48 {
                Self::add_to_classpath(libraries_dir, classpath, &path, &file)?;
            }
            info!("Built in forge library, skipping...");
            return Ok(());
        }

        let url = if let Some(downloads) = &library.downloads {
            downloads.artifact.url.clone()
        } else {
            let baseurl = if let Some(url) = &library.url {
                url.to_owned()
            } else {
                "https://libraries.minecraft.net/".to_owned()
            };
            format!("{baseurl}{path}/{file}")
        };

        let lib_dir_path = libraries_dir.join(&path);
        std::fs::create_dir_all(&lib_dir_path).path(&lib_dir_path)?;

        let dest = lib_dir_path.join(&file);
        let dest_str = dest
            .to_str()
            .ok_or(ForgeInstallError::PathBufToStr(dest.clone()))?;

        info!(
            "Installing forge: Downloading library ({}/{num_libraries}): {}",
            library_i + 1,
            library.name
        );

        self.send_progress(ForgeInstallProgress::P5DownloadingLibrary {
            num: library_i + 1,
            out_of: num_libraries,
        });

        if dest.exists() {
            info!("Library already exists.");
        } else {
            match file_utils::download_file_to_bytes(&self.client, &url, false).await {
                Ok(bytes) => {
                    std::fs::write(&dest, bytes).path(dest)?;
                }
                Err(err) => {
                    err!("Error downloading library: {err}\n        Trying pack.xz version");
                    let result = self.unpack_augmented_library(dest_str, &url).await;
                    if result.is_not_found() {
                        err!("Error 404 not found. Skipping...");
                        return Ok(());
                    }
                    result?;
                }
            };
        }

        Self::add_to_classpath(libraries_dir, classpath, &path, &file)?;

        Ok(())
    }

    fn add_to_classpath(
        libraries_dir: &Path,
        classpath: &mut String,
        path: &str,
        file: &str,
    ) -> Result<(), ForgeInstallError> {
        let classpath_item = libraries_dir.join(format!("{path}/{file}{CLASSPATH_SEPARATOR}"));
        // println!("adding library to classpath {classpath_item:?}");
        classpath.push_str(
            classpath_item
                .to_str()
                .ok_or(ForgeInstallError::PathBufToStr(classpath_item.clone()))?,
        );
        Ok(())
    }

    async fn unpack_augmented_library(
        &self,
        dest_str: &str,
        url: &str,
    ) -> Result<(), ForgeInstallError> {
        pt!("Unpacking augmented library");
        pt!("Downloading File");
        let bytes =
            file_utils::download_file_to_bytes(&self.client, &format!("{url}.pack.xz"), false)
                .await?;
        pt!("Extracting pack.xz");
        let temp_extract_xz = Self::extract_zip_file(&bytes)?;

        pt!("Reading signature");
        let extracted_pack_path = temp_extract_xz.path().join(format!("{dest_str}.pack"));
        let mut extracted_pack =
            std::fs::File::open(&extracted_pack_path).path(&extracted_pack_path)?;
        extracted_pack
            .seek(std::io::SeekFrom::End(-8))
            .path(&extracted_pack_path)?;
        let mut sig_len_bytes = [0u8; 4];
        extracted_pack
            .read_exact(&mut sig_len_bytes)
            .path(&extracted_pack_path)?;
        let sig_len = u32::from_le_bytes(sig_len_bytes);

        let full_len = std::fs::metadata(&extracted_pack_path)
            .path(&extracted_pack_path)?
            .len();
        let crop_len = full_len - sig_len as u64 - 8;

        let extracted_pack =
            std::fs::File::open(&extracted_pack_path).path(&extracted_pack_path)?;
        let mut pack_crop = Vec::with_capacity(crop_len as usize);
        extracted_pack
            .take(crop_len)
            .read_to_end(&mut pack_crop)
            .path(extracted_pack_path)?;

        let cropped_pack_path = temp_extract_xz
            .path()
            .join(format!("{dest_str}.pack.crop",));
        std::fs::write(&cropped_pack_path, &pack_crop).path(cropped_pack_path)?;

        pt!("Unpacking extracted file");
        let unpack200_path = get_java_binary(JavaVersion::Java8, "unpack200", None).await?;
        let output = Command::new(&unpack200_path)
            .args(&[format!("{dest_str}.pack.crop",), dest_str.to_owned()])
            .output()
            .path(unpack200_path)?;

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
                    .ok_or(ForgeInstallError::PathBufToStr(parent.clone()))?
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

    pub fn extract_zip_file(archive: &[u8]) -> Result<tempfile::TempDir, ForgeInstallError> {
        let temp_dir = match tempfile::TempDir::new() {
            Ok(temp_dir) => temp_dir,
            Err(err) => return Err(ForgeInstallError::TempFile(err)),
        };

        let target_dir = std::path::PathBuf::from(temp_dir.path());

        zip_extract::extract(std::io::Cursor::new(archive), &target_dir, true)?;

        Ok(temp_dir)
    }
}

async fn get_forge_version(minecraft_version: &str) -> Result<String, ForgeInstallError> {
    let json = JsonForgeVersions::download().await?;
    let version = json
        .get_forge_version(minecraft_version)
        .ok_or(ForgeInstallError::NoForgeVersionFound)?;
    Ok(version)
}

fn get_instance_dir(instance_name: &InstanceSelection) -> Result<PathBuf, ForgeInstallError> {
    let instance_dir = file_utils::get_instance_dir(instance_name)?;
    Ok(instance_dir)
}

fn get_forge_dir(instance_dir: &Path) -> Result<PathBuf, ForgeInstallError> {
    let forge_dir = instance_dir.join("forge");
    std::fs::create_dir_all(&forge_dir).path(&forge_dir)?;
    Ok(forge_dir)
}

fn create_mods_dir(instance_dir: &Path) -> Result<(), ForgeInstallError> {
    let mods_dir_path = instance_dir.join(".minecraft/mods");
    std::fs::create_dir_all(&mods_dir_path).path(mods_dir_path)?;
    Ok(())
}

fn create_lock_file(instance_dir: &Path) -> Result<(), ForgeInstallError> {
    let lock_path = instance_dir.join("forge.lock");
    std::fs::write(
        &lock_path,
        "If you see this, forge was not installed correctly.",
    )
    .path(lock_path)?;
    Ok(())
}

fn get_minecraft_version(instance_dir: &Path) -> Result<String, ForgeInstallError> {
    let version_json_path = instance_dir.join("details.json");
    let version_json = std::fs::read_to_string(&version_json_path).path(version_json_path)?;
    let version_json = serde_json::from_str::<VersionDetails>(&version_json)?;
    let minecraft_version = version_json.id;
    Ok(minecraft_version)
}

pub async fn install_w(
    instance_name: InstanceSelection,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<GenericProgress>>,
) -> Result<(), String> {
    match instance_name {
        InstanceSelection::Instance(name) => install_client(name, f_progress, j_progress).await,
        InstanceSelection::Server(name) => install_server(name, j_progress, f_progress).await,
    }
    .map_err(|err| err.to_string())
}

pub async fn install_client_w(
    instance_name: String,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<GenericProgress>>,
) -> Result<(), String> {
    install_client(instance_name, f_progress, j_progress)
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

pub async fn install_client(
    instance_name: String,
    f_progress: Option<Sender<ForgeInstallProgress>>,
    j_progress: Option<Sender<GenericProgress>>,
) -> Result<(), ForgeInstallError> {
    info!("Started installing forge");

    if let Some(progress) = &f_progress {
        let _ = progress.send(ForgeInstallProgress::P1Start);
    }

    let installer = ForgeInstaller::new(
        f_progress,
        InstanceSelection::Instance(instance_name.clone()),
    )
    .await?;

    let (installer_file, installer_name, installer_path) =
        installer.download_forge_installer().await?;

    let (libraries_dir, mut classpath) = installer
        .run_installer_and_get_classpath(&installer_name, installer_path, j_progress)
        .await?;

    let mut clean_classpath = String::new();

    let forge_json = ForgeInstaller::get_forge_json(&installer_file)?;

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
        installer
            .download_library(
                library,
                library_i,
                num_libraries,
                &libraries_dir,
                &mut classpath,
                &mut clean_classpath,
            )
            .await?;
    }

    let classpath_path = installer.forge_dir.join("classpath.txt");
    std::fs::write(&classpath_path, &classpath).path(classpath_path)?;

    let clean_classpath_path = installer.forge_dir.join("clean_classpath.txt");
    std::fs::write(&clean_classpath_path, &clean_classpath).path(clean_classpath_path)?;

    let json_path = installer.forge_dir.join("details.json");
    std::fs::write(&json_path, serde_json::to_string(&forge_json)?).path(json_path)?;

    change_instance_type(&installer.instance_dir, "Forge".to_owned()).await?;

    installer.remove_lock()?;
    info!("Finished installing forge");
    Ok(())
}
