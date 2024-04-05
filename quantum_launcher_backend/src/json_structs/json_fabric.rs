use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct FabricJSON {
    pub mainClass: String,
    pub arguments: Arguments,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Arguments {
    pub jvm: Vec<String>,
}
