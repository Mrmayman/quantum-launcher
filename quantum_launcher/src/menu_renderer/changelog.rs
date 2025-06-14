use std::sync::LazyLock;

use iced::{
    widget::{self, image::Handle},
    Length,
};

use crate::{
    config::LauncherConfig,
    icon_manager,
    state::{AccountMessage, MenuWelcome, Message, NEW_ACCOUNT_NAME},
};

use super::{button_with_icon, center_x, get_themes_and_styles, Element, DISCORD};

pub static IMG_LAUNCHER: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../quantum_launcher.png").as_slice())
});
pub static IMG_NEW: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/new.png").as_slice())
});
pub static IMG_LOADERS: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/install_loader.png").as_slice())
});
pub static IMG_MOD_STORE: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/mod_store.png").as_slice())
});
pub static IMG_OLD_MC: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/old_mc.png").as_slice())
});
pub static IMG_THEMES: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/themes.png").as_slice())
});
pub static IMG_PRESETS: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/screenshots/presets.png").as_slice())
});
pub static IMG_LOGO: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../assets/icon/ql_logo.png").as_slice())
});

pub fn changelog_0_4_1<'a>() -> Element<'a> {
    widget::column![
        widget::text("QuantumLauncher v0.4.1 is here!").size(40),

        widget::container(
            widget::column![
                widget::text("TL;DR").size(32),
                "- Shaders, Resource Pack, Modpack stores added",
                "- Modpack support is here!",
                "- Quality-of-life tweaks, keyboard navigation",
                "- Portable mode, custom location support via qldir.txt",
                "- Major bugfixes across modding, authentication, and platform compatibility",
            ].padding(10).spacing(5)
        ),

        widget::Space::with_height(5),
        widget::horizontal_rule(4.0),
        widget::text("Modding").size(32),
        "- added Modpacks, Shaders and Resource Packs stores",
        "- drag-and-drop support for mod files, modpacks and presets",
        "- jarmod patching support (useful for old Minecraft versions)",
        "- OptiFine support for b1.6.6, b1.7.3, 1.2.5 and 1.5.2",
        widget::text("UX").size(24),
        "- \"Open Mod Page\" in mod description now shows URL in tooltip",
        "- added bullet/numbered list support in curseforge mod descriptions",
        "- mod update checking is now up to 3-4x faster!",

        widget::horizontal_rule(4.0),
        widget::text("Portable Mode & Custom Paths").size(32),
        "- You can control where QuantumLauncher stores data by placing a qldir.txt file in one of the following locations:",
        "-- Next to the launcher executable",
        "-- In the working directory where it's run",
        "-- In the global [QuantumLauncher folder](https://mrmayman.github.io/quantumlauncher/faq#files-location)",

        "- Leave the file blank for full portable mode, or write a path inside it to set a custom data location.",
        "- > Note: On macOS you may need to extract the .app from the DMG for portable mode.",

        widget::horizontal_rule(4.0),
        widget::text("Menu Overhauls").size(32),
        "- Welcome screen redesigned to guide new users more clearly",
        "- Error screen looks better, now has a \"Copy Log + Error\" button",
        "- Fabric Install screen is now simpler and less cluttered",

        widget::horizontal_rule(4.0),
        widget::text("UI/UX").size(32),
        "- new Catppuccin color scheme",
        "- improved aesthetics of light themes",
        "- animated pulsating \"...\" dots in a few \"Loading\" screens",
        "- minor tweaks to buttons and layout in many screens",
        "- added a \"Clean Java Installs\" button (for reinstalling java) in launcher settings",
        "- old versions in the version list now have cleaner names (\"beta/b1.7/b1.7.3\" -> \"b1.7.3\")",
        "- window size now respects UI scale",

        widget::horizontal_rule(4.0),
        widget::text("Keyboard Navigation").size(32),
        "- Up/Down arrow keys to select instance",
        "- Ctrl+Enter -> launch instance",
        "- Ctrl+Backspace -> kill instance",
        "- Ctrl+Q -> exit launcher (only in main menu, when nothing's running)",
        "- (note: on macOS it's Cmd instead of Ctrl)",
        "- More coming in the future!",

        widget::horizontal_rule(4.0),
        widget::text("Game Crashed message").size(32),
        "- there is now a \"Join Discord\" button in Logs tab",
        "- there's no longer a tip mentioning a \"second terminal\" upon crash (no longer needed)",

        widget::horizontal_rule(4.0),
        widget::text("Dependencies (Technical)").size(32),
        "- dropped OpenSSL and LZMA dependencies on linux",
        "-- now depends only on libgcc_s and libc (glibc)",
        "-- replaced OpenSSL with rustls",

        widget::horizontal_rule(4.0),
        widget::text("Fixes").size(32),
        "- crash messages now appear in Logs tab (critical!)",
        "- forge now works after renaming instance",
        "- multiplayer now works in 1.16.x for offline accounts",
        "- mod updating and preset importing are now working again! (fixed error for curseforge, index corruption for modrinth)",
        "- fixed extreme launcher log file spam",
        widget::text("UI/UX").size(24),
        "- UI scaling now has a minimum limit preventing unusable layouts",
        "- many mod-related operations now return you to Mods screen, instead of launch screen",
        "- pressing Esc in mod descriptions now exits the description only (not the entire store)",
        "- fixed mod store \"forgetting\" scroll position when opening description and exiting",
        widget::text("Platform-Specific").size(24),
        "- fixed 1.13-1.16 crashing on linux ARM",
        "- fixed 1.8.9 crashing on macOS ARM",
        "- fixed Java 8 (1.16.5 and below) compatibility on Windows ARM and macOS",
    ].padding(10).spacing(5).into()
}

#[allow(unused)]
pub fn changelog_0_4<'a>() -> Element<'a> {
    widget::column!(
        widget::text("QuantumLauncher v0.4").size(40),
        widget::text("Changelog:").size(30),
        widget::text("Redesign:").size(20),
        "- Redesigned the launcher menus!",
        widget::image(IMG_LAUNCHER.clone()).width(400),
        "- Select instances easily with a sidebar, and enjoy the simpler navigation with tabs!",
        "- The purple colour pallete has been changed to be more vibrant and punchy",
        "- Many other menus have been changed to look nicer",
        widget::image(IMG_PRESETS.clone()).width(400),
        widget::text("Mods:").size(20),
        "- Added CurseForge to the Mods store (alongside Modrinth). You can choose the backend",
        widget::image(IMG_MOD_STORE.clone()).width(400),
        "- Added NeoForge installer!",
        widget::image(IMG_LOADERS.clone()).width(400),
        "- Mod updating and preset importing is now nearly 2x faster!",
        "- Getting list of versions when creating instance is now nearly 2x faster!",
        "- The mod store now has infinite scrolling! Just scroll down to see more mods",
        /*widget::text("Servers").size(20),
        "- Added a server management system",
        "- You can create, edit, delete, launch and install mods for servers",
        "- Extensive configuration and server plugin management!"*/ // Postponed
        widget::text("Microsoft Account").size(20),
        "- Added optional Microsoft login for those with a paid account",
        "- Normal users can continue using the launcher, this is entirely optional",
        widget::text("Other").size(20),
        "- Added option to close launcher after opening game",
        "- Overhauled the game log viewer. There is no limit now!",
        "- You can see the launcher debug logs by clicking the bottom bar",
        "- macOS support is now ready!",
        "- Redesigned the command-line experience with the \"clap\" library",
        "- Improved Java installer support for Windows 32 bit, Windows ARM and macOS",
        "- MASSIVE code cleanup and overhaul",
        "- Updated \"iced\" to 0.13.1 from 0.12.1",
        widget::text("Fixes").size(20),
        "- Fixed the terminal popping up on Windows",
        "- Fixed a bug where you couldn't disable local mods",
        "- Fixed a JSON error when installing some mods (like debugify)",
        "- Fixed a bug where renaming instances would break mod management features and forge",
        "- Fixed a crash with 1.21.3 Fabric",
        "- Fixed a crash with 1.21.5 Forge",
        "- Fixed many rendering bugs with the mod description viewer in the mod store",
        "- Reduced useless log file spam in \"QuantumLauncher/logs/\" folder",
        "- Reduced errors in the fabric installer",
        "- Fixed many bugs with game log viewing",
        "- Improved reliability of Network Operations, you should see \"Request Error\"s less now",
    )
    .spacing(7)
    .width(Length::Fill)
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

impl MenuWelcome {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        match self {
            MenuWelcome::P1InitialScreen => widget::column![
                widget::vertical_space(),
                center_x(widget::image(IMG_LOGO.clone()).width(200)),
                center_x(widget::text("Welcome to QuantumLauncher!").size(20)),
                center_x(widget::button("Get Started").on_press(Message::WelcomeContinue1)),
                widget::vertical_space(),
            ]
            .align_x(iced::alignment::Horizontal::Center)
            .spacing(10)
            .into(),
            MenuWelcome::P2Theme => {
                let (theme, style) = get_themes_and_styles(config);
                widget::column![
                    widget::vertical_space(),
                    center_x(widget::text("Customize your launcher!").size(24)),
                    widget::row![
                        widget::horizontal_space(),
                        "Select Theme:",
                        theme,
                        widget::horizontal_space(),
                    ]
                    .spacing(10),
                    widget::row![
                        widget::horizontal_space(),
                        "Select Color Scheme:",
                        style,
                        widget::horizontal_space(),
                    ]
                    .spacing(10),
                    widget::Space::with_height(5),
                    center_x("Oh, and also..."),
                    center_x(
                        button_with_icon(icon_manager::chat(), "Join our Discord", 16)
                            .on_press(Message::CoreOpenLink(DISCORD.to_owned()))
                    ),
                    widget::Space::with_height(5),
                    center_x(widget::button("Continue").on_press(Message::WelcomeContinue2)),
                    widget::vertical_space(),
                ]
                .spacing(10)
                .into()
            }
            MenuWelcome::P3Auth => widget::column![
                widget::vertical_space(),
                center_x(
                    widget::text_input("Enter username...", &config.username)
                        .width(200)
                        .on_input(Message::LaunchUsernameSet)
                ),
                center_x(
                    widget::button(center_x("Continue"))
                        .width(200)
                        .on_press_maybe((!config.username.is_empty()).then_some(
                            Message::LaunchScreenOpen {
                                message: None,
                                clear_selection: true
                            }
                        ))
                ),
                widget::Space::with_height(10),
                center_x(widget::text("OR").size(20)),
                widget::Space::with_height(10),
                center_x(
                    widget::button("Login to Microsoft").on_press(Message::Account(
                        AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned())
                    ))
                ),
                widget::vertical_space(),
            ]
            .spacing(5)
            .into(),
        }
    }
}

#[allow(unused)]
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
                    Message::CoreOpenLink(DISCORD.to_owned())
                ),
            ).padding(10).spacing(10)
        ),
        "Happy Gaming!",
        widget::button("Continue").on_press(Message::LaunchScreenOpen { message: None, clear_selection: true })
    ).padding(10).spacing(10)).into()
}
