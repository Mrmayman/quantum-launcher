use ql_instances::{
    file_utils, io_err,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
};

use crate::launcher_state::{Launcher, MenuModsDownload, State};

impl Launcher {
    pub fn open_mods_screen(&mut self) -> Result<(), String> {
        let launcher_dir = file_utils::get_launcher_dir().map_err(|err| err.to_string())?;

        let selected_instance = self
            .selected_instance
            .as_ref()
            .ok_or("No instance selected")?;

        let instances_dir = launcher_dir.join("instances").join(selected_instance);

        let config_path = instances_dir.join("config.json");
        let config = std::fs::read_to_string(&config_path)
            .map_err(io_err!(config_path))
            .map_err(|err| err.to_string())?;
        let config: InstanceConfigJson =
            serde_json::from_str(&config).map_err(|err| err.to_string())?;

        let version_path = instances_dir.join("details.json");
        let version = std::fs::read_to_string(&version_path)
            .map_err(io_err!(version_path))
            .map_err(|err| err.to_string())?;
        let version: VersionDetails =
            serde_json::from_str(&version).map_err(|err| err.to_string())?;

        self.state = State::ModsDownload(MenuModsDownload {
            query: String::new(),
            results: None,
            config,
            json: version,
        });
        Ok(())
    }
}
