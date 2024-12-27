mod download;
mod instance;
pub mod json_profiles;
mod launcher_update_detector;

pub use download::constants::OS_NAME;
pub use instance::create::{create_instance, create_instance_w};
pub use instance::launch::{launch, launch_w, AssetRedownloadProgress, GameLaunchResult};
pub use instance::list_versions::list_versions;
pub use instance::read_log::{read_logs, read_logs_w, LogEvent, LogLine, LogMessage, ReadError};
pub use launcher_update_detector::{
    check_for_launcher_updates, check_for_launcher_updates_w, install_launcher_update,
    install_launcher_update_w, UpdateCheckInfo, UpdateError, UpdateProgress,
};
pub use omniarchive_api::{ListEntry, ScrapeProgress};

use semver::{BuildMetadata, Prerelease};

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 3,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

pub const LAUNCHER_VERSION_NAME: &str = "0.3";
