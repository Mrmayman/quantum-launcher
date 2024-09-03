# quantum-launcher
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
- On *Windows*, the launcher files are at `C:/Users/YOURUSERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- The launcher configuration, including manually added Java versions and default username, is at `QuantumLauncher/config.json`.
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.

# Todo
- Add java installs dropdown list for Java override selection.
- Add ability to enable and disable mods.
- Add menu to see logs.
- Autodownload for Forge, Quilt and OptiFine.
- A built in mod store using the Modrinth API.
- Managing your aternos servers from within the launcher.
- A local server hosting system (without port forwarding).
- macOS support.

# Note on Piracy
If you pirate the game, it's at your own risk. I am not responsible for any issues caused. I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.