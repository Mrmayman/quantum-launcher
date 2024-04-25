pub(crate) const VERSIONS_JSON: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[cfg(target_os = "linux")]
pub const OS_NAME: &str = "linux";

#[cfg(target_os = "windows")]
pub const OS_NAME: &str = "windows";

#[cfg(target_os = "macos")]
pub const OS_NAME: &str = "osx";

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub const OS_NAME: &str = "unknown";

pub const DEFAULT_RAM_MB_FOR_INSTANCE: usize = 2048;
