use std::path::Path;
use ql_instances::import_export::{import,export};
use ql_instances::import_export::import::import_instance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = Path::new("/home/sreehari/.config/QuantumLauncher/idk.zip");
    // import_instance(&zip_path,false).await;
    let test_config = import::InstanceInfo{instance_name :String::from("craftmine") , instance_version : String::from("1.21.5"),exeption: vec![String::from("Hello")] };
    export::export_instance(test_config, zip_path,None);
    Ok(())
}