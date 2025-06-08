use std::path::Path;

use ql_instances::import_export::import::import_instance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = Path::new("/home/sreehari/.config/QuantumLauncher/idk.zip");
    import_instance(&zip_path,true).await;
    Ok(())
}