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

### Added versions to version list:
Snapshots of 1.0 to 1.5.2 are now available in the version list

**1.0**:  1.0.0-rc1, 1.0.0-rc2

**1.1**:    11w47a, 11w48a, 11w49a, 11w50a, 12w01a

**1.2**:    1.2-pre, 1.2.5-pre, 12w03a, 12w04a, 12w05a-1354,
            12w05a-1442, 12w05b, 12w06a, 12w07a, 12w07b, 12w08a

**1.3**:    1.3-pre, 1.3.1-pre, 1.3.2-pre, 12w15a, 12w16a,
            12w17a, 12w18a, 12w19a, 12w21a, 12w21b, 12w22a,
            12w23a, 12w23b, 12w24a, 12w25a, 12w26a, 12w27a,
            12w30a, 12w30b, 12w30c, 12w30d, 12w30e

**1.4**:    1.4-pre, 1.4.1-pre, 1.4.3-pre, 1.4.5-pre, 1.4.6-pre,
            12w32a, 12w34a, 12w34b, 12w36a, 12w37a, 12w38a, 12w38b,
            12w39a-1243, 12w39b, 12w40a, 12w40b, 12w41a, 12w41b,
            12w42a, 12w42b, 12w49a, 12w50a, 12w50b

**1.5**:    1.5-pre, 1.5.1-pre, 1.5.2-pre, 13w01a, 13w01b, 13w02a,
            13w02b, 13w03a, 13w04a, 13w05a, 13w05b, 13w06a, 13w07a,
            13w09a, 13w09b, 13w09c, 13w10a, 13w10b, 13w11a, 13w12~
