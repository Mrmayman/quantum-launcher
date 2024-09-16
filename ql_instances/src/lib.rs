mod download;
pub mod error;
pub mod file_utils;
mod instance;
pub mod java_install;
pub mod json_structs;

pub use download::progress::DownloadProgress;
pub use instance::create::create_instance;
pub use instance::launch::{launch, launch_wrapped, GameLaunchResult};
pub use instance::list_versions::list_versions;
pub use instance::read_log::{
    read_logs, read_logs_wrapped, LogEvent, LogLine, LogMessage, ReadError,
};
pub use java_install::JavaInstallProgress;

use semver::{BuildMetadata, Prerelease};

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 2,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

const LAUNCHER_VERSION_NAME: &str = "0.2";
