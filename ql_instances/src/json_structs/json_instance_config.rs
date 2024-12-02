use serde::{Deserialize, Serialize};

/// Configuration for a specific instance.
///
/// # Fields
///
/// ## `mod_type`
/// Can be one of:
/// - `"Vanilla"`
/// - `"Fabric"`
/// - `"Forge"` (coming soon)
/// - `"OptiFine"`
/// - `"Quilt"` (coming soon)
///
/// ## `java_override`
/// DEPRECATED: NO FUNCTIONALITY
///
/// ## `ram_in_mb`
/// The amount of RAM in megabytes the instance should have.
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceConfigJson {
    pub java_override: Option<String>,
    pub ram_in_mb: usize,
    pub mod_type: String,
    pub enable_logger: Option<bool>,
}

impl InstanceConfigJson {
    /// Returns the amount of RAM in megabytes as a String.
    ///
    /// This is the format that the Java arguments understand.
    pub fn get_ram_in_string(&self) -> String {
        format!("{}M", self.ram_in_mb)
    }

    /// Returns a String containing the Java argument to
    /// allocate the configured amount of RAM.
    pub fn get_ram_argument(&self) -> String {
        format!("-Xmx{}", self.get_ram_in_string())
    }
}
