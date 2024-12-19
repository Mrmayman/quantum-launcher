mod download;
mod instance;
pub mod json_structs;
mod launcher_update_detector;

pub use download::{constants::OS_NAME, progress::DownloadProgress, DownloadError};
pub use instance::create::{create_instance, create_instance_wrapped};
pub use instance::launch::{launch, launch_wrapped, AssetRedownloadProgress, GameLaunchResult};
pub use instance::list_versions::{list_versions, ListEntry};
pub use instance::read_log::{
    read_logs, read_logs_wrapped, LogEvent, LogLine, LogMessage, ReadError,
};
pub use launcher_update_detector::{
    check_for_launcher_updates, check_for_launcher_updates_wrapped, install_launcher_update,
    install_launcher_update_wrapped, UpdateCheckInfo, UpdateError, UpdateProgress,
};

use semver::{BuildMetadata, Prerelease};

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 3,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

pub const LAUNCHER_VERSION_NAME: &str = "0.3";
