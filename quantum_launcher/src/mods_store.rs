use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use iced::Task;
use ql_core::{
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    InstanceSelection, IntoIoError, IntoStringError, Loader,
};
use ql_mod_manager::mod_manager::{ModIndex, Query, Search};

use crate::launcher_state::{InstallModsMessage, Launcher, MenuModsDownload, Message, State};

impl Launcher {
    pub fn open_mods_screen(&mut self) -> Result<Task<Message>, String> {
        let selection = self.selected_instance.as_ref().unwrap();
        let instances_dir = selection.get_instance_path(&self.dir);

        let config_path = instances_dir.join("config.json");
        let config = std::fs::read_to_string(&config_path)
            .path(config_path)
            .strerr()?;
        let config: InstanceConfigJson = serde_json::from_str(&config).strerr()?;

        let version_path = instances_dir.join("details.json");
        let version = std::fs::read_to_string(&version_path)
            .path(version_path)
            .strerr()?;
        let version: VersionDetails = serde_json::from_str(&version).strerr()?;

        let mod_index = ModIndex::get_s(selection).strerr()?;

        let mut menu = MenuModsDownload {
            config,
            json: version,
            is_loading_search: false,
            latest_load: Instant::now(),
            query: String::new(),
            results: None,
            opened_mod: None,
            result_data: HashMap::new(),
            mods_download_in_progress: HashSet::new(),
            mod_index,
        };
        let command = menu.search_modrinth(matches!(
            &self.selected_instance,
            Some(InstanceSelection::Server(_))
        ));
        self.state = State::ModsDownload(Box::new(menu));
        Ok(command)
    }
}

impl MenuModsDownload {
    pub fn search_modrinth(&mut self, is_server: bool) -> Task<Message> {
        let Ok(loaders) = Loader::try_from(self.config.mod_type.as_str()) else {
            return Task::none();
        };

        self.is_loading_search = true;
        let query = Query {
            name: self.query.clone(),
            versions: vec![self.json.id.clone()],
            loaders: vec![loaders],
            server_side: is_server,
            open_source: false, // TODO: Add Open Source filter
        };
        Task::perform(async move { Search::search(query).await.strerr() }, |n| {
            Message::InstallMods(InstallModsMessage::SearchResult(n))
        })
    }
}
