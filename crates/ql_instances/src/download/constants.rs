#[cfg(feature = "simulate_linux_arm64")]
pub const OS_NAME: &str = "linux";
#[cfg(not(feature = "simulate_linux_arm64"))]
mod os_name {
    #[cfg(target_os = "linux")]
    pub const OS_NAME: &str = "linux";
    #[cfg(target_os = "windows")]
    pub const OS_NAME: &str = "windows";
    #[cfg(target_os = "macos")]
    pub const OS_NAME: &str = "osx";
    #[cfg(target_os = "freebsd")]
    pub const OS_NAME: &str = "freebsd";
}
#[cfg(not(feature = "simulate_linux_arm64"))]
pub use os_name::OS_NAME;

pub const DEFAULT_RAM_MB_FOR_INSTANCE: usize = 2048;

#[cfg(feature = "simulate_linux_arm64")]
pub const OS_NAMES: &[&str] = &["linux"];
#[cfg(not(feature = "simulate_linux_arm64"))]
mod os_names {
    #[cfg(target_os = "linux")]
    pub const OS_NAMES: &[&str] = &["linux"];
    #[cfg(target_os = "windows")]
    pub const OS_NAMES: &[&str] = &["windows"];
    #[cfg(target_os = "macos")]
    pub const OS_NAMES: &[&str] = &["macos", "osx"];
    #[cfg(target_os = "freebsd")]
    pub const OS_NAMES: &[&str] = &["freebsd"];
}
#[cfg(not(feature = "simulate_linux_arm64"))]
pub use os_names::OS_NAMES;

#[cfg(target_arch = "arm")]
pub const ARCH: &str = "arm32";
#[cfg(any(target_arch = "aarch64", feature = "simulate_linux_arm64"))]
pub const ARCH: &str = "arm64";
#[cfg(target_arch = "x86")]
pub const ARCH: &str = "x86";
