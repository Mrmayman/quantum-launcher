use ql_core::{file_utils, pt, DownloadFileError, IntoIoError, LAUNCHER_DIR};

/// Gets the java argument to start the authlib injector.
///
/// Authlib Injector allows the game to use alternative
/// authentication methods such as <https://ely.by>.
///
/// This function automatically downloads it from
/// [GitHub](https://github.com/yushijinhun/authlib-injector)
/// and sets it up if not present, and then returns
/// `-javaagent:YOUR_LAUNCHER_DIR/downloads/authlib_injector.jar=ely.by`
pub async fn get_authlib_injector() -> Result<String, DownloadFileError> {
    const URL: &str = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";

    let dir = LAUNCHER_DIR.join("downloads");
    tokio::fs::create_dir_all(&dir).await.path(&dir)?;

    let path = dir.join("authlib_injector.jar");
    if !path.is_file() {
        pt!("Downloading authlib-injector...");
        file_utils::download_file_to_path(URL, false, &path).await?;
    }

    Ok(format!("-javaagent:{}=ely.by", path.to_string_lossy()))
}
