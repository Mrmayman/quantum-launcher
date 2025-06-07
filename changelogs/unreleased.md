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
- snapshots of 1.0 to 1.5.2 are now available in the version list

## Refactored portable dir system
Create a `qldir.txt` file, put it either in:
- The current directory
- Next to the executible
- In the default QuantumLauncher location

### 1) Portable mode: Single-folder structure
You can leave the file empty for portable mode
(put it in current dir or next to executable)

Here the portable mode file structure will be:
```
your_dir/
    quantum_launcher.exe
    qldir.txt
    QuantumLauncher/
        instances/
        logs/
        config.json
```

### 2) Portable mode: Flattened structure
You can put a `.` in the file for a different
flattened file structure in portable mode.

```
your_dir/
    quantum_launcher.exe
    qldir.txt
    instances/
    logs/
    config.json
```

### 3) Custom paths
You can put a custom path in the file
to store the data in any custom location
(like an external drive or a Games folder).

### Flags (optional)
On the second line of the qldir.txt file
(which is optional), you can specify comma-separated
flags:

- `top`: Used as for flattened structure (2)
  with any paths, not just `.`

- `i_vulkan`: Force vulkan graphics for the
  launcher's interface
- `i_opengl`: Force OpenGL graphics for the
  launcher's interface
- `i_directx` (windows-only): Force DirectX 12
  graphics for the launcher's interface
- `i_metal` (macOS-only): Force Metal graphics
  for the launcher's interface

## Other
- Improved support for weird character encodings in file paths
