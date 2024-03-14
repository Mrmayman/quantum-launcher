use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct VersionDetails {
    arguments: Value,
    assetIndex: Value,
    assets: String,
    complianceLevel: usize,
    downloads: Value,
    id: String,
    javaVersion: Value,
    libraries: Vec<Value>,
    logging: Value,
    mainClass: String,
    minimumLauncherVersion: usize,
    releaseTime: String,
    time: String,
    r#type: String,
}
