# quantum-launcher-rs
A minimalistic Minecraft launcher written in *Rust* with the *iced* framework for Windows and Linux.

![Preview](quantum_launcher.png)

# Features
- Instances to isolate different installations. No more conflicts between versions!
- Install fabric with one click.
- Create or delete an Instance easily.
- Autoinstalls Java for you.
## Assets
- Choose not to download assets (saving space)
- Download assets to a centralized location, never downloading them twice.

# Location
- On *Windows*, the launcher files are at `AppData/Roaming/QuantumLauncher/`.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher`.
- The launcher configuration, including manually added Java versions and default username, is at `QuantumLauncher/config.json`.
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.

# Todo
- Add java installs dropdown list for Java override selection.
- Add ability to enable and disable mods.
- Add menu to see logs.
- Fix the weird freeze when downloading assets on Windows.
- Autodownload for Forge, Quilt and OptiFine.
- A built in mod store using the Modrinth API.
- Managing your aternos servers from within the launcher.
- macOS support.

# Note on Piracy
(For legal reasons) I do not encourage or promote piracy in any way. Authentication is not implemented for ease of development and use. If you use this launcher and pirate the game, you are playing at your own risk.

If anyone has any complaint, open an issue in the repo and I will try my best to address it.