use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmcPack {
    pub components: Vec<MmcPackComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct MmcPackComponent {
    pub cachedName: String,
    pub version: String,
}
