use iced::widget;

use super::Element;

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
