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
- All versions before 1.21.6 will now support skins from elyby by default (b)
- You can also login with elyby accounts now!

## Instance Packaging
- You can now package entire QuantumLauncher instances into a single file
- Import/Export support for Prism Launcher/MultiMC instances
- (TODO) Migration from other launchers

## Fixes
- fixed Minecraft Indev and early Infdev being unplayable (b)
- fixed many crashes on Linux ARM and macOS (b)
- fixed broken colors in old versions on M-series Macs (b)
- fixed getting stuck in an infinite loop when downloading some curseforge mods
- fixed Fabric API being missing for some curseforge mods
- fixed game crashes in portable mode
- fixed java install progress bar being stuck at the end
- fixed many formatting issues in game logs

- old Minecraft versions are now in the correct order in the download list (b)
- snapshots of 1.0 to 1.5.2 are no longer missing for download (b)
- performance of loading the version list
  (when clicking New button) is **way** better now (b)
- made a few cryptic errors more understandable
- improved support for weird character encodings in file paths
- missing libraries are now auto-downloaded
