use iced::widget::{self, image::Handle};
use lazy_static::lazy_static;

use crate::{icon_manager, launcher_state::Message};

use super::{button_with_icon, Element};

lazy_static! {
    pub static ref IMG_NEW: Handle =
        Handle::from_memory(include_bytes!("../../../assets/screenshots/new.png"));
    pub static ref IMG_LOADERS: Handle = Handle::from_memory(include_bytes!(
        "../../../assets/screenshots/install_loader.png"
    ));
    pub static ref IMG_MOD_STORE: Handle =
        Handle::from_memory(include_bytes!("../../../assets/screenshots/mod_store.png"));
    pub static ref IMG_OLD_MC: Handle =
        Handle::from_memory(include_bytes!("../../../assets/screenshots/old_mc.png"));
    pub static ref IMG_THEMES: Handle =
        Handle::from_memory(include_bytes!("../../../assets/screenshots/themes.png"));
}

pub fn changelog_0_3_1<'a>() -> Element<'a> {
    widget::column!(
        widget::text("QuantumLauncher v0.3.1").size(32),
        widget::text("Changelog:").size(20),
        "- Added Quilt support",
        "- Added Omniarchive integration (downloading old versions of Minecraft)",
        "- Added support for Linux ARM64 (early alpha)",
        "- Improved command line support (try quantum_launcher --help for more info)",
        "- Added a changelog viewer (which you're seeing right now lol)",
        widget::text("Mods:").size(20),
        "- Added presets! You can now share your mod configurations with others",
        "- Now you can copy id or open the mod page in the mod store",
        widget::text("Bugfixes:").size(20),
        "- Launcher logs no longer have garbled text on windows",
        "- Launcher logs are no longer delayed on windows",
        "- Fixed a crash in some old versions of Minecraft (useLegacyMergeSort)",
        "- Fixed a bug where OptiFine installer was broken on windows"
    )
    .spacing(10)
    .into()
}

pub fn welcome_msg<'a>() -> Element<'a> {
    widget::scrollable(widget::column!(
        widget::text("Welcome to QuantumLauncher!").size(32),
        "A simple, effortless Minecraft Launcher",
        "- Create instances of Minecraft by pressing \"New\"",
        widget::image(IMG_NEW.clone()).width(200),
        "- Edit instance settings (such as Java path, memory allocation and arguments) by selecting your instance and pressing \"Edit\"",
        widget::text("Modding").size(20),
        "- Install fabric, forge, optifine, or quilt by selecting your instance and pressing \"Mods->Install Fabric (or whatever you want)\"",
        widget::image(IMG_LOADERS.clone()).width(200),
        "- Browse the endless collections of mods through the built in mod store at \"Mods->Download Mods\"",
        widget::image(IMG_MOD_STORE.clone()).width(300),
        "- Package up your mods and send them to your friends (or download recommended ones) at \"Mods->Presets\"",
        widget::text("...and much more!").size(20),
        "- Skin and sound fixes for old Minecraft versions",
        "- Omniarchive integration (to download old, rare versions of Minecraft)",
        widget::image(IMG_OLD_MC.clone()),
        "- Say goodbye to worrying about installing Java: it's all automated!",
        "- Fast, lightweight and responsive (unlike some... other launchers)",
        "- Customizable themes and styles!",
        widget::image(IMG_THEMES.clone()),
        widget::container(
            widget::column!(
                "Got any problems? Join the discord!",
                button_with_icon(icon_manager::chat(), "Join our Discord").on_press(
                    Message::CoreOpenDir("https://discord.gg/bWqRaSXar5".to_owned())
                ),
            ).padding(10).spacing(10)
        ),
        "Happy Gaming!",
        widget::button("Continue").on_press(Message::LaunchScreenOpen { message: None, clear_selection: true })
    ).padding(10).spacing(10)).into()
}
