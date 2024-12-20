#[cfg(target_os = "linux")]
pub const OS_NAME: &str = "linux";

#[cfg(target_os = "windows")]
pub const OS_NAME: &str = "windows";

#[cfg(target_os = "macos")]
pub const OS_NAME: &str = "osx";

pub const DEFAULT_RAM_MB_FOR_INSTANCE: usize = 2048;
