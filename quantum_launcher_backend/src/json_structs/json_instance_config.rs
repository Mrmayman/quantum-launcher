use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InstanceConfigJson {
    pub java_override: Option<String>,
    pub ram: u64,
}

impl InstanceConfigJson {
    pub fn get_ram_in_string(&self) -> String {
        format!("{}M", self.ram)
    }
}
