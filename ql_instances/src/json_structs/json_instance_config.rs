use serde::{Deserialize, Serialize};

/// Configuration for a specific instance.
///
/// # Fields
///
/// ## `mod_type`
/// Can be one of:
/// - `"Vanilla"`
/// - `"Fabric"`
/// - `"Forge"`
/// - `"OptiFine"`
/// - `"Quilt"` (coming soon)
///
/// ## `java_override`
/// If you want to use your own Java installation
/// instead of the auto-installed one, specify
/// the path to the `java` executable here.
///
/// ## `ram_in_mb`
/// The amount of RAM in megabytes the instance should have.
///
/// ## `enable_logger`
/// - `true` (default): Show log output in launcher.
///   May not show all log output, especially during a crash.
/// - `false`: Print raw, unformatted log output to the console.
///   This is useful for debugging, but may be hard to read.
///
/// ## `java_args`, `game_args`
/// These are optional lists of additional
/// arguments to pass to Java and the game.
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceConfigJson {
    pub mod_type: String,
    pub java_override: Option<String>,
    pub ram_in_mb: usize,
    pub enable_logger: Option<bool>,
    pub java_args: Option<Vec<String>>,
    pub game_args: Option<Vec<String>>,
}

impl InstanceConfigJson {
    fn get_ram_in_string(&self) -> String {
        format!("{}M", self.ram_in_mb)
    }

    /// Returns a String containing the Java argument to
    /// allocate the configured amount of RAM.
    pub fn get_ram_argument(&self) -> String {
        format!("-Xmx{}", self.get_ram_in_string())
    }
}
