use std::{io::Cursor, path::Path, sync::mpsc::Sender};

use ql_core::{file_utils, GenericProgress};

use crate::{extract_tar_gz, send_progress, JavaInstallError, JavaVersion};

pub async fn install_third_party_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    install_dir: &Path,
) -> Result<(), JavaInstallError> {
    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    let url = {
        match version {
            JavaVersion::Java16 |
            JavaVersion::Java17 |
            JavaVersion::Java21 => {
                return Err(JavaInstallError::UnsupportedOnlyJava8)
            },
            JavaVersion::Java8 => "https://github.com/hmsjy2017/get-jdk/releases/download/v8u231/jdk-8u231-linux-arm32-vfp-hflt.tar.gz",
        }
    };
    #[cfg(not(all(target_os = "linux", target_arch = "arm")))]
    let url = version.get_corretto_url();

    send_progress(
        java_install_progress_sender,
        GenericProgress {
            done: 0,
            total: 2,
            message: Some("Getting tar.gz archive".to_owned()),
            has_finished: false,
        },
    );
    let file_bytes = file_utils::download_file_to_bytes(url, false).await?;
    send_progress(
        java_install_progress_sender,
        GenericProgress {
            done: 1,
            total: 2,
            message: Some("Extracting tar.gz archive".to_owned()),
            has_finished: false,
        },
    );
    if url.ends_with("tar.gz") {
        extract_tar_gz(&file_bytes, install_dir).map_err(JavaInstallError::TarGzExtract)?;
    } else if url.ends_with("zip") {
        zip_extract::extract(Cursor::new(&file_bytes), install_dir, true)?;
    } else {
        return Err(JavaInstallError::UnknownExtension(url.to_owned()));
    }
    Ok(())
}
