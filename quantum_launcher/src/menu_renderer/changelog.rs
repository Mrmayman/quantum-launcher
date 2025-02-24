use iced::widget::{self, image::Handle};
use lazy_static::lazy_static;

use crate::{icon_manager, launcher_state::Message};

use super::{button_with_icon, Element, DISCORD};

lazy_static! {
    pub static ref IMG_NEW: Handle =
        Handle::from_bytes(include_bytes!("../../../assets/screenshots/new.png").as_slice());
    pub static ref IMG_LOADERS: Handle = Handle::from_bytes(
        include_bytes!("../../../assets/screenshots/install_loader.png").as_slice()
    );
    pub static ref IMG_MOD_STORE: Handle =
        Handle::from_bytes(include_bytes!("../../../assets/screenshots/mod_store.png").as_slice());
    pub static ref IMG_OLD_MC: Handle =
        Handle::from_bytes(include_bytes!("../../../assets/screenshots/old_mc.png").as_slice());
    pub static ref IMG_THEMES: Handle =
        Handle::from_bytes(include_bytes!("../../../assets/screenshots/themes.png").as_slice());
}

pub fn changelog_0_4<'a>() -> Element<'a> {
    widget::column!(
        widget::text("QuantumLauncher v0.4").size(40),
        widget::text("Changelog:").size(30),
        widget::text("Redesign:").size(20),
        "- The launcher menus have been redesigned",
        "- Select instances easily with a sidebar, and enjoy the simpler navigation with tabs!",
        "- The purple colour pallete has been changed to be more vibrant and punchy",
        widget::text("Plugins:").size(20), // TODO: Implement this
        "- Added a lua-based plugin system. Tweak the launcher to your liking",
        "- There's now a plugin store too!",
        "- Plugins are safe. They are completely sandboxed and manually reviewed for security",
        widget::text("Servers").size(20),
        "- Added a server management system",
        "- You can create, edit, delete, launch and install mods for servers",
        "- Extensive configuration and server plugin management!", // TODO: Implement this too and stop yapping
        widget::text("Microsoft Account").size(20),
        "- Added optional Microsoft login for those with a paid account",
        "- Normal users can continue using the launcher, this is entirely optional",
        widget::text("Other").size(20),
        "- Redesigned the command-line experience with the \"clap\" library",
        "- Updated \"iced\" to 0.13.1 from 0.12.1",
        // Look, these aren't false promises. By the time the update releases
        // I will either have finished these or removed them from changelog
    )
    .spacing(10)
    .into()
}

#[allow(unused)]
pub fn changelog_0_3_1<'a>() -> Element<'a> {
    widget::column!(
        widget::text("QuantumLauncher v0.3.1").size(32),
        "Your soon-to-be favorite launcher just got even better!",
        widget::text("Changelog:").size(20),
        "- Quilt support is here!",
        widget::image(IMG_LOADERS.clone()).width(200),
        "- Added instance renaming",
        "- Omniarchive integration is here, allowing you to download old, rare versions of Minecraft. Nostalgia incoming!",
        widget::image(IMG_OLD_MC.clone()),
        "- Added a brand-new style: Sky blue. Try it out in Settings -> Style!",
        widget::image(IMG_THEMES.clone()),
        "- Added a cool ASCII art intro in the terminal",
        widget::text("Mods:").size(20),
        "- Added mod presets!",
        "-- Share your custom mod configurations with friends",
        "-- Download recommended mods directly from the launcher",
        "- Now you can copy mod IDs or open the mod page in the mod store",
        widget::text("Misc:").size(20),
        "- Improved command line support (try quantum_launcher --help for more info)",
        "- Added a changelog viewer (You're looking at it right now lol)",
        "- Added a welcome screen for new users! Say hi!",
        "- Started work on packaging (Flatpak, Deb, RPM)",
        "- Added (experimental) support for Linux ARM64 (early alpha)",
        "- Started work on macOS support. Hello there, you mac users!",
        "-- If you have a mac and are willing to test the launcher,",
        "   feel free to download the macOS beta from the website and try it out",
        "- Added a confirmation dialog for uninstalling loaders",
        "- MASSIVE codebase cleanup, optimizations and improvements (you probably won't notice it)",
        widget::text("Fixes and polish:").size(20),
        "- Launcher logs no longer have garbled text on windows",
        "- Launcher logs are no longer delayed on windows",
        "- Fixed a crash in some old versions of Minecraft (useLegacyMergeSort)",
        "- Fixed a really dumb bug where OptiFine installer was broken on windows"
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
                button_with_icon(icon_manager::chat(), "Join our Discord", 16).on_press(
                    Message::CoreOpenDir(DISCORD.to_owned())
                ),
            ).padding(10).spacing(10)
        ),
        "Happy Gaming!",
        widget::button("Continue").on_press(Message::LaunchScreenOpen { message: None, clear_selection: true })
    ).padding(10).spacing(10)).into()
}
