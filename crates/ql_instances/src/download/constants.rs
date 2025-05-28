#[cfg(target_os = "linux")]
pub const OS_NAME: &str = "linux";

#[cfg(target_os = "windows")]
pub const OS_NAME: &str = "windows";

#[cfg(target_os = "macos")]
pub const OS_NAME: &str = "osx";

#[cfg(target_os = "freebsd")]
pub const OS_NAME: &str = "freebsd";

pub const DEFAULT_RAM_MB_FOR_INSTANCE: usize = 2048;

#[cfg(target_os = "linux")]
pub const OS_NAMES: &[&str] = &["linux"];

#[cfg(target_os = "windows")]
pub const OS_NAMES: &[&str] = &["windows"];

#[cfg(target_os = "macos")]
pub const OS_NAMES: &[&str] = &["macos", "osx"];

#[cfg(target_os = "freebsd")]
pub const OS_NAMES: &[&str] = &["freebsd"];

#[cfg(target_arch = "arm")]
pub const ARCH: &str = "arm32";
#[cfg(target_arch = "aarch64")]
pub const ARCH: &str = "arm64";
