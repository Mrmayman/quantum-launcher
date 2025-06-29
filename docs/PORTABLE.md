This document describes how QuantumLauncher's
portable mode configuration can be used.

Create a `qldir.txt` file, put it either in:
- The current directory
- Next to the executible
- In the default QuantumLauncher location

## 1) Portable mode: Single-folder structure
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

## 2) Portable mode: Flattened structure
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

## 3) Custom paths
You can put a custom path in the file
to store the data in any custom location
(like an external drive or a Games folder).

## Flags (optional)
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
