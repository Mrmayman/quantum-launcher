//! # A module for creating, managing and running Minecraft client instances
//!
//! This module contains functions to:
//! - Create and delete an instance
//! - Launch the instance
//! - Update the launcher
//! - Read logs
//! - List versions available for download
//!
//! # A rant about natives
//! ## What are natives?
//! Natives are libraries that are platform-specific.
//! They are used by Minecraft to interface with the operating system.
//!
//! ## Types of natives
//! - `natives: *` - These are part of the main library
//!   but have a separate jar file included that is extracted to
//!   the `natives` folder.
//! - `name: *-natives-*` - They are a separate library
//!   whose jar file is extracted to the `natives` folder.
//! - `classifiers: *` - Once again, part of main library
//!   but have separate jar files for each OS. Just formatted
//!   differently in the json.
//!
//! These 3 separate types of natives make it a headache to
//! deal with all three correctly, WHILE JUGGLING ALONG
//! THIRD PARTY ARM64 SOURCES FOR LIBRARIES!!!
//!
//! ## The problem
//! Mojang has a habit of not including ARM64 natives in their
//! libraries (well they do sometimes but not always).
//!
//! This is a problem for ARM64 users as they can't
//! run the game without the natives.
//!
//! ## The solution
//! We download the ARM64 natives from two different sources:
//! - `./assets/lwjgl_arm64/*` - Providing natives for different LWJGL
//!   versions. Sourced from <https://github.com/theofficialgman/piston-meta-arm64>
//! - `./assets/minecraft_arm` - Providing natives for LWJGL,
//!   Log4J, Oshi, JavaObjCBridge, and Slf4J. Used less often.
//!   Sourced from <https://github.com/Kichura/Minecraft_ARM>
//!
//! Both of these complement each other and provide a complete
//! set of natives for ARM64 users.
//!
//! It's still a bit of a hack and it sometimes breaks but it works.

mod download;
mod instance;
mod json_natives;
mod json_profiles;
mod launcher_update_detector;

pub use download::constants::OS_NAME;
pub use instance::create::{create_instance, create_instance_w};
pub use instance::launch::{launch, launch_w, GameLaunchResult};
pub use instance::list_versions::list_versions;
pub use instance::read_log::{read_logs, read_logs_w, LogEvent, LogLine, LogMessage, ReadError};
pub use launcher_update_detector::{
    check_for_launcher_updates, check_for_launcher_updates_w, install_launcher_update,
    install_launcher_update_w, UpdateCheckInfo, UpdateError,
};
pub use omniarchive_api::ListEntry;

use semver::{BuildMetadata, Prerelease};

const LAUNCHER_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 3,
    patch: 1,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};
