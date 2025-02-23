use std::{path::PathBuf, process::Stdio};

use ql_core::{file_utils, IntoIoError};
use tokio::{
    io::AsyncBufReadExt,
    process::{Child, Command},
};

use crate::ServerError;

async fn get_ssh_path() -> Result<(PathBuf, Command), ServerError> {
    if cfg!(target_os = "windows") {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let ssh_dir = launcher_dir.join("ssh");
        let program = ssh_dir.join("ssh.exe");
        if program.exists() {
            return Ok((program, Command::new(ssh_dir.join("ssh.exe"))));
        }

        tokio::fs::create_dir_all(&ssh_dir)
            .await
            .path(ssh_dir.clone())?;

        let client = reqwest::Client::new();

        let url = if cfg!(target_arch = "x86_64") {
            "https://github.com/PowerShell/Win32-OpenSSH/releases/download/v9.8.1.0p1-Preview/OpenSSH-Win64.zip"
        } else if cfg!(target_arch = "x86") {
            "https://github.com/PowerShell/Win32-OpenSSH/releases/download/v9.8.1.0p1-Preview/OpenSSH-Win32.zip"
        } else if cfg!(target_arch = "aarch64") {
            "https://github.com/PowerShell/Win32-OpenSSH/releases/download/v9.8.1.0p1-Preview/OpenSSH-ARM64.zip"
        } else if cfg!(target_arch = "arm") {
            "https://github.com/PowerShell/Win32-OpenSSH/releases/download/v9.8.1.0p1-Preview/OpenSSH-ARM.zip"
        } else {
            return Err(ServerError::UnsupportedSSHArchitecture);
        };

        let archive = file_utils::download_file_to_bytes(url, false).await?;

        zip_extract::extract(std::io::Cursor::new(&archive), &ssh_dir, true)?;

        Ok((program.clone(), Command::new(program)))
    } else {
        Ok((PathBuf::from("ssh"), Command::new("ssh")))
    }
}

pub async fn run_tunnel(port: u16) -> Result<(Child, String), ServerError> {
    let (path, mut cmd) = get_ssh_path().await?;

    let mut cmd = cmd
        .args(&["-R", &format!("0:localhost:{port}"), "serveo.net"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .path(path)?;

    let reader_stdout = tokio::io::BufReader::new(cmd.stdout.take().unwrap());
    let reader_stderr = tokio::io::BufReader::new(cmd.stderr.take().unwrap());

    let lines_stdout = reader_stdout.lines();
    let lines_stderr = reader_stderr.lines();

    todo!()
}
