use std::{path::PathBuf, process::Command};

use crate::error::{LauncherError, LauncherResult};

#[derive(Debug)]
pub struct JavaInstall {
    pub version: usize,
    path: PathBuf,
}

impl JavaInstall {
    /// Finds all the installations of java on your computer and returns an Vec<JavaInstall>.
    ///
    /// JavaInstall has a version and path field.
    /// # What it does
    /// It has to turn:
    ///
    /// ```
    /// openjdk version "1.8.0_402"
    /// OpenJDK Runtime Environment (Zulu 8.76.0.17-CA-win64) (build 1.8.0_402-b06)
    /// OpenJDK 64-Bit Server VM (Zulu 8.76.0.17-CA-win64) (build 25.402-b06, mixed mode)
    /// ```
    ///
    /// into `8` (the java version)
    pub fn find_java_installs(
        manually_added: Option<&[String]>,
    ) -> LauncherResult<Vec<JavaInstall>> {
        let mut paths: Vec<JavaInstall> = if let Some(n) = manually_added {
            n.iter().map(|n| get_java_install(n)).collect()
        } else {
            Ok(vec![])
        }?;

        let main_paths: LauncherResult<Vec<JavaInstall>> =
            get_java_paths()?.lines().map(get_java_install).collect();
        let main_paths = main_paths?;

        paths.extend(main_paths);
        Ok(paths)
    }

    pub fn get_command(&self) -> Command {
        Command::new(&self.path)
    }
}

fn get_java_install(path: &str) -> LauncherResult<JavaInstall> {
    let path = PathBuf::from(path);
    let java_output = Command::new(&path)
        .arg("-version")
        .output()
        .map_err(LauncherError::CommandError)?;
    let java_output = String::from_utf8(java_output.stderr)?;

    let first_line = java_output
        .lines()
        .next()
        .ok_or(LauncherError::JavaVersionIsEmptyError)?;

    let version = extract_version_string_from_message(first_line).ok_or(
        LauncherError::JavaVersionImproperVersionPlacement(first_line.to_owned()),
    )?;

    let number = get_number_from_version_string(version).ok_or(
        LauncherError::JavaVersionParseToNumberError(version.to_owned()),
    )?;

    Ok(JavaInstall {
        version: number,
        path,
    })
}

fn get_number_from_version_string(s: &str) -> Option<usize> {
    let parts: Vec<&str> = s.split('.').collect();
    if let Some(first_part) = parts.first() {
        if first_part == &"1" {
            // If starts with "1.", get the second number
            // java 1.8.0 -> 8
            if let Some(second_part) = parts.get(1) {
                return second_part.parse().ok();
            }
        } else {
            // Otherwise, get the first number
            // java 17.0 -> 17
            return first_part.parse().ok();
        }
    }
    None
}

fn get_java_paths() -> LauncherResult<String> {
    match String::from_utf8(
        match std::env::consts::OS {
            "windows" => Command::new("where")
                .arg("java")
                .output()
                .map_err(LauncherError::CommandError)?,
            "macos" => Command::new("/usr/libexec/java_home")
                .output()
                .map_err(LauncherError::CommandError)?,
            _ if cfg!(unix) => Command::new("which")
                .arg("java")
                .output()
                .map_err(LauncherError::CommandError)?,
            _ => panic!("OS not supported for finding java"),
        }
        .stdout,
    ) {
        Ok(n) => Ok(n),
        Err(n) => Err(LauncherError::from(n)),
    }
}

fn extract_version_string_from_message(message: &str) -> Option<&str> {
    let start_idx = message.find(|c: char| c.is_numeric() || c == '.')?;
    let end_idx = message[start_idx..].find(|c: char| !c.is_numeric() && c != '.')?;

    Some(&message[start_idx..start_idx + end_idx])
}
