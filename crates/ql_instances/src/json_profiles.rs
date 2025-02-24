use std::collections::BTreeMap;

use serde::Serialize;

/// Represents the `launcher_profiles.json` file.
///
/// It's not needed for the game to run, but some
/// loader installers depend on it so it's included.
#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct ProfileJson {
    pub profiles: BTreeMap<String, Profiles>,
    pub clientToken: Option<String>,
    // Map<UUID, AuthenticationDatabase>
    pub authenticationDatabase: Option<BTreeMap<String, AuthenticationDatabase>>,
    pub launcherVersion: Option<LauncherVersion>,
    pub settings: Settings,
    pub analyticsToken: Option<String>,
    pub analyticsFailcount: Option<i32>,
    pub selectedUser: Option<SelectedUser>,
    pub version: Option<i32>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct Profiles {
    pub name: String,
    pub r#type: Option<String>,
    pub created: Option<String>,
    pub lastUsed: Option<String>,
    pub icon: Option<String>,
    pub lastVersionId: String,
    pub gameDir: Option<String>,
    pub javaDir: Option<String>,
    pub javaArgs: Option<String>,
    pub logConfig: Option<String>,
    pub logConfigIsXML: Option<bool>,
    pub resolution: Option<Resolution>,
}

#[derive(Serialize)]
pub struct Resolution {
    pub height: i32,
    pub width: i32,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct AuthenticationDatabase {
    pub accessToken: String,
    pub username: String,
    // Map<UUID, Name>
    pub profiles: BTreeMap<String, String>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct LauncherVersion {
    pub name: String,
    pub format: i32,
    pub profilesFormat: i32,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub enableSnapshots: bool,
    pub enableAdvanced: bool,
    pub keepLauncherOpen: bool,
    pub showGameLog: bool,
    pub locale: Option<String>,
    pub showMenu: bool,
    pub enableHistorical: bool,
    pub profileSorting: String,
    pub crashAssistance: bool,
    pub enableAnalytics: bool,
    pub soundOn: Option<bool>,
}

#[derive(Serialize)]
pub struct SelectedUser {
    pub account: String,
    pub profile: String,
}

impl Default for ProfileJson {
    fn default() -> Self {
        Self {
            profiles: [].into(),
            clientToken: None,
            authenticationDatabase: None,
            launcherVersion: None,
            settings: Settings {
                enableSnapshots: true,
                enableAdvanced: true,
                keepLauncherOpen: true,
                showGameLog: true,
                locale: None,
                showMenu: true,
                enableHistorical: true,
                profileSorting: "ByLastPlayed".to_owned(),
                crashAssistance: false,
                enableAnalytics: false,
                soundOn: Some(false),
            },
            analyticsToken: None,
            analyticsFailcount: None,
            selectedUser: None,
            version: None,
        }
    }
}
