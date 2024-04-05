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
/// - `"Optifine"` (coming soon)
/// - `"Quilt"` (coming soon)
///
/// ## `java_override`
/// If you want to force the instance to use a
/// specific Java version, you can specify it here.
///
/// ## `ram_in_mb`
/// The amount of RAM in megabytes the instance should have.
#[derive(Serialize, Deserialize)]
pub struct InstanceConfigJson {
    pub java_override: Option<String>,
    pub ram_in_mb: usize,
    pub mod_type: String,
}

impl InstanceConfigJson {
    /// Returns the amount of RAM in megabytes as a string.
    ///
    /// This is the format that the Java arguments understand.
    pub fn get_ram_in_string(&self) -> String {
        format!("{}M", self.ram_in_mb)
    }
}
