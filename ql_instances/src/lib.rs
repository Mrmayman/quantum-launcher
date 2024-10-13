mod download;
pub mod error;
pub mod file_utils;
mod instance;
pub mod java_install;
pub mod json_structs;
mod launcher_update_detector;
pub mod locks;
pub mod print;

pub use download::constants::OS_NAME;
pub use download::progress::DownloadProgress;
pub use instance::create::{create_instance, create_instance_wrapped};
pub use instance::launch::{launch, launch_wrapped, GameLaunchResult};
pub use instance::list_versions::list_versions;
pub use instance::read_log::{
    read_logs, read_logs_wrapped, LogEvent, LogLine, LogMessage, ReadError,
};
pub use java_install::JavaInstallProgress;
pub use launcher_update_detector::{
    check_for_updates, check_for_updates_wrapped, install_update, install_update_wrapped,
    UpdateCheckInfo, UpdateError, UpdateProgress,
};
pub use locks::{MOD_DOWNLOAD_LOCK, RATE_LIMITER};

use semver::{BuildMetadata, Prerelease};

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 2,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

pub const LAUNCHER_VERSION_NAME: &str = "0.2";
