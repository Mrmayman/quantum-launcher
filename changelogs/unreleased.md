# Unreleased

## Switch to BetterJSONs and LaunchWrapper
- the launcher now uses [BetterJSONs](https://github.com/MCPHackers/BetterJSONs/)
  for downloading instances, and [LaunchWrapper](https://github.com/MCPHackers/LaunchWrapper)
  for running old Minecraft versions
- Many fixes and improvements have been made as a result,
  they will be marked with (b).

---

- Overhauled portable dir system (see `docs/PORTABLE.md` for more info)
- Added a new Teal color scheme!
- Overhauled the Launcher Setings menu

## Elyby integration
- Minecraft 1.21.5 and below will now support skins from elyby by default (b)
- You can also login with elyby accounts now!

## Instance Packaging
- You can now package entire QuantumLauncher instances into a single file
- Import/Export support for Prism Launcher/MultiMC instances
- (TODO) Migration from other launchers

# UI
- Revamped all icons in the launcher (thanks, [Aurlt](https://github.com/Aurlt) !)
- Overhauled launcher settings menu
- Added a licenses page

## Fixes
- Fixed Minecraft Indev and early Infdev being unplayable (b)
- Fixed many crashes on Linux ARM and macOS (b)
- Fixed broken colors in old versions on M-series Macs (b)
- Fixed getting stuck in an infinite loop when downloading some curseforge mods
- Fixed Fabric API being missing for some curseforge mods
- Fixed game crashes in portable mode
- Fixed java install progress bar being stuck at the end
- Fixed many formatting issues in game logs
- Fixed welcome screen not working

- Old Minecraft versions are now in the correct order in the download list (b)
- Snapshots of 1.0 to 1.5.2 are no longer missing for download (b)
- Performance of loading the version list
  (when clicking New button) is **way** better now (b)
- Improved readability of a few errors
- Improved support for weird character encodings in file paths
- Missing libraries are now auto-downloaded
