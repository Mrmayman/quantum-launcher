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

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
mod lwjgl {
    pub const LWJGL_294: &str =
        include_str!("../../../../assets/lwjgl_arm64/2.9.4-nightly-20150209.json");
    pub const LWJGL_312: &str = include_str!("../../../../assets/lwjgl_arm64/3.1.2.json");
    pub const LWJGL_316: &str = include_str!("../../../../assets/lwjgl_arm64/3.1.6.json");
    pub const LWJGL_321: &str = include_str!("../../../../assets/lwjgl_arm64/3.2.1.json");
    pub const LWJGL_322: &str = include_str!("../../../../assets/lwjgl_arm64/3.2.2.json");
    pub const LWJGL_331: &str = include_str!("../../../../assets/lwjgl_arm64/3.3.1.json");
    pub const LWJGL_332: &str = include_str!("../../../../assets/lwjgl_arm64/3.3.2.json");
    pub const LWJGL_333: &str = include_str!("../../../../assets/lwjgl_arm64/3.3.3.json");
}

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub use lwjgl::*;
