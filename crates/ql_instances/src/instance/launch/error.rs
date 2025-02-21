use std::path::PathBuf;
use thiserror::Error;

use ql_core::{json::VersionDetails, DownloadError, IoError, JavaInstallError, JsonFileError};

use crate::mc_auth::AuthError;

#[derive(Debug, Error)]
pub enum GameLaunchError {
    #[error(transparent)]
    Io(#[from] IoError),
    #[error(transparent)]
    DownloadError(#[from] DownloadError),
    #[error("username contains spaces")]
    UsernameHasSpaces,
    #[error("username is empty")]
    UsernameIsEmpty,
    #[error(transparent)]
    JsonFile(#[from] JsonFileError),
    #[error("instance not found")]
    InstanceNotFound,
    #[error("semver error: {0}")]
    Semver(#[from] semver::Error),
    #[error("no arguments field in details.json")]
    VersionJsonNoArgumentsField(Box<VersionDetails>),
    #[error("couldn't convert PathBuf to string: {0:?}")]
    PathBufToString(PathBuf),
    #[error(transparent)]
    JavaInstall(#[from] JavaInstallError),
    #[error("couldn't run java command: {0}")]
    CommandError(std::io::Error),
    #[error("error upgrading forge install (transforming path)\n{FORGE_UPGRADE_MESSAGE}")]
    ForgeInstallUpgradeTransformPathError,
    #[error("error upgrading forge install (removing prefix)\n{FORGE_UPGRADE_MESSAGE}")]
    ForgeInstallUpgradeStripPrefixError,
    #[error(transparent)]
    MsAuth(#[from] AuthError),
}

const FORGE_UPGRADE_MESSAGE: &str = r"outdated forge install. Please uninstall and reinstall.
Select your instance, go to Mods -> Uninstall Forge, then Install Forge.";
