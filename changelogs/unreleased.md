# Unreleased
## Switch to BetterJSONs and LaunchWrapper
- the launcher now uses [BetterJSONs](https://github.com/MCPHackers/BetterJSONs/)
  for downloading instances, and [LaunchWrapper](https://github.com/MCPHackers/LaunchWrapper)
  for running old Minecraft versions
### As a result...
- performance of loading version list (when clicking New button)
  is **way** better now
- old Minecraft versions are now in the correct order
- Minecraft Indev and early Infdev now work properly
- library downloading for ARM on Linux and macOS has been overhauled
  (for better or for worse)
- colors for old Minecraft versions are no longer glitched on M1 mac
