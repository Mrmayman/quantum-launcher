use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS");

    if let Ok(target_os) = target_os {
        if target_os == "windows" {
            println!("cargo::rerun-if-changed=../assets/icon/ql_logo.rc");
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            let manifest_dir = PathBuf::from(manifest_dir);
            let manifest_dir = manifest_dir.parent().unwrap();

            let icon_dir = manifest_dir.join("assets/icon/ql_logo.rc");

            embed_resource::compile(&icon_dir, embed_resource::NONE)
                .manifest_optional()
                .unwrap();
        }
    }
}
