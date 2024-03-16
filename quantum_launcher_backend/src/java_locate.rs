use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{LauncherError, LauncherResult};

// This code is ugly and I accept it.

#[derive(Debug)]
pub struct JavaInstall {
    pub version: usize,
    pub path: PathBuf,
}

impl From<(usize, PathBuf)> for JavaInstall {
    fn from(value: (usize, PathBuf)) -> Self {
        JavaInstall {
            version: value.0,
            path: value.1,
        }
    }
}

impl JavaInstall {
    pub fn find_java_installs() -> LauncherResult<Vec<JavaInstall>> {
        let java_paths = get_java_paths()?;
        // This code is a monster, but it's necessary.
        // It has to turn:

        // openjdk version "1.8.0_402"
        // OpenJDK Runtime Environment (Zulu 8.76.0.17-CA-win64) (build 1.8.0_402-b06)
        // OpenJDK 64-Bit Server VM (Zulu 8.76.0.17-CA-win64) (build 25.402-b06, mixed mode)

        // into 8 (the java version)
        java_paths
            .lines()
            .map(PathBuf::from)
            // Runs command, gets first line of the output mentioning java version.
            .map(|path| {
                let version_message = get_java_version_message(&path);
                let first_line = version_message.and_then(|n| {
                    n.lines()
                        .next()
                        .map(|n| n.to_owned())
                        .ok_or(LauncherError::JavaVersionIsEmptyError)
                });
                (first_line, path)
            })
            // Cut out 1.8.0_402 from "openjdk version\" 1.8.0_402\"".
            .map(|(version_line, path)| {
                let version = version_line.and_then(|n| {
                    extract_number(&n)
                        .map(|n| n.to_owned())
                        .ok_or(LauncherError::JavaVersionImproperVersionPlacement(n))
                });
                (version, path)
            })
            // Get the 8 in 1.8.0_402
            // Eg: 8 from 1.8.0_402
            .map(|(version_name, path)| {
                (
                    version_name.and_then(|version| {
                        get_version_number(&version).ok_or(
                            LauncherError::JavaVersionParseToNumberError(version.clone()),
                        )
                    }),
                    path,
                )
            })
            .map(|tuple| tuple.0.and_then(|t| Ok((t, tuple.1))))
            .map(|tuple| tuple.map(JavaInstall::from))
            .collect()
    }
}

fn get_version_number(s: &str) -> Option<usize> {
    let parts: Vec<&str> = s.split('.').collect();
    if let Some(first_part) = parts.get(0) {
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
            "windows" => Command::new("where").arg("java").output()?,
            "macos" => Command::new("/usr/libexec/java_home").output()?,
            _ if cfg!(unix) => Command::new("which").arg("java").output()?,
            _ => panic!("OS not supported for finding java"),
        }
        .stdout,
    ) {
        Ok(n) => Ok(n),
        Err(n) => Err(LauncherError::from(n)),
    }
}

fn get_java_version_message(path: &Path) -> LauncherResult<String> {
    match Command::new(path).arg("-version").output() {
        Ok(n) => match String::from_utf8(n.stderr) {
            Ok(n) => Ok(n),
            Err(n) => Err(LauncherError::from(n)),
        },
        Err(n) => Err(LauncherError::from(n)),
    }
}

fn extract_number(message: &str) -> Option<&str> {
    let start_idx = message.find(|c: char| c.is_numeric() || c == '.')?;
    let end_idx = message[start_idx..].find(|c: char| !c.is_numeric() && c != '.')?;

    Some(&message[start_idx..start_idx + end_idx])
}
