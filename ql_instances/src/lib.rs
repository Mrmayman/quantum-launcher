mod download;
pub mod error;
pub mod file_utils;
mod instance;
pub mod java_install;
pub mod json_structs;
mod launcher_update_detector;
pub mod print;

use std::time::Duration;

use lazy_static::lazy_static;

pub use download::constants::OS_NAME;
pub use download::progress::DownloadProgress;
pub use instance::create::create_instance;
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

use semver::{BuildMetadata, Prerelease};
use tokio::sync::Mutex;
use tokio::time::Instant;

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 2,
    patch: 0,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

pub const LAUNCHER_VERSION_NAME: &str = "0.2";

lazy_static! {
    pub static ref RATE_LIMITER: RateLimiter = RateLimiter::default();
    pub static ref MOD_DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());
}

pub struct RateLimiter {
    last_executed: Mutex<Instant>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            last_executed: Mutex::new(Instant::now() - Self::DELAY),
        }
    }
}

impl RateLimiter {
    // 200ms delay duration
    const DELAY: Duration = Duration::from_millis(200);

    pub async fn lock(&self) {
        let mut last_exec_time = self.last_executed.lock().await;
        let now = Instant::now();

        let elapsed = now.duration_since(*last_exec_time);

        if elapsed < Self::DELAY {
            let wait_duration = Self::DELAY - elapsed;
            tokio::time::sleep(wait_duration).await;
        }

        // Update the last execution time to now
        *last_exec_time = Instant::now();
    }
}
