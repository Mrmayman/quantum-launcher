use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

#[derive(Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
}
