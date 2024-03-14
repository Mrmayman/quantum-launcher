use std::{fs::File, io::Write};

use crate::{
    download::GameDownloader,
    error::{LauncherError, LauncherResult},
    file_utils,
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub fn launch(instance_name: &str) -> LauncherResult<()> {
    let launcher_dir = file_utils::get_launcher_dir()?;

    let instances_dir = launcher_dir.join("instances");
    file_utils::create_dir_if_not_exists(&instances_dir)?;

    if !instances_dir.join(instance_name).exists() {
        return Err(LauncherError::InstanceNotFound);
    }

    // let mut class_path: String = "".to_owned();

    // let libraries = get!(version_json["libraries"].as_array(), "version.libraries");

    // for library in libraries {
    //     let library_path = get!(
    //         library["downloads"]["artifact"]["path"].as_str(),
    //         "version.libraries[].downloads.artifact.path"
    //     );
    //     let library_path = instance_dir.join("libraries").join(library_path);
    //     if library_path.exists() {
    //         class_path.push_str(
    //             library_path
    //                 .to_str()
    //                 .expect("Could not append library to classpath"),
    //         );
    //         class_path.push(CLASSPATH_SEPARATOR);
    //     }
    // }

    todo!()
}

pub fn create(instance_name: &str, version: String) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    let game_downloader = GameDownloader::new(instance_name, &version)?;
    game_downloader.download_jar()?;
    game_downloader.download_libraries()?;
    game_downloader.download_logging_config()?;
    game_downloader.download_assets()?;

    let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
    json_file.write_all(game_downloader.version_json.to_string().as_bytes())?;

    Ok(())
}
