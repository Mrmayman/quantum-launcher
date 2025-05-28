# Unreleased
## BetterJSONs
- the launcher now uses [BetterJSONs](https://github.com/MCPHackers/BetterJSONs/)
  for downloading instances, so now the omniarchive website
  is no longer scraped for links
### As a result...
- performance of loading version list (when clicking New button)
  is **way** better now
- old Minecraft versions are now in the correct order
- Minecraft indev and early infdev now work properly thanks
  to the use of LaunchWrapper
- library downloading for ARM on Linux and macOS has been overhauled
  (for better or for worse)
