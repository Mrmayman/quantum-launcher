# quantum-launcher-rs
A minimalistic Minecraft launcher written in *Rust* with the *iced* framework for Windows and Linux.

![Preview](quantum_launcher.png)

# Features
- Instances to isolate different installations. No more conflicts between versions!
- Create or delete an Instance with ease.
- Launch your game with one click.
- Remembers your username across sessions.
- Can't autodetect java? Add it manually with a simple file picker.

# Location
- On *Windows*, the launcher files are at `AppData/Roaming/QuantumLauncher/`.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher`.
- The launcher configuration, including manually added Java versions and default username, is at `QuantumLauncher/config.json`.
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.

# Todo
- macOS support.
- Fix many bugs and quality of life issues.
- Fix instability on Windows. (my main dev machine is Linux `:)` ) 
- Separate Java versions for each instance.
- Manual override of Java versions.
- Autodownload for Forge, Fabric and OptiFine.
- A built in mod store using the Modrinth API.
- Managing your aternos servers from within the launcher.

# Note on Piracy
(For legal reasons) I do not encourage or promote piracy in any way. Authentication is not implemented for ease of development and use. If you use this launcher and pirate the game, you are playing at your own risk.

If anyone has any complaint, open an issue in the repo and I will address it.