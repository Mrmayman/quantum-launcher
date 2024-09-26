# quantum-launcher
A minimalistic Minecraft launcher written in *Rust* with the *iced* framework for Windows and Linux.

![Preview](quantum_launcher.png)

# Features
- Instances to isolate different installations. No more conflicts between versions!
- Install fabric with one click.
- Create or delete an Instance easily.
- Autoinstalls Java for you.
- Lightweight and responsive. The launcher opens pretty much instantly (unlike some minute-long loading screens on _other launchers_).

# Location
- On *Windows*, the launcher files are at `C:/Users/YOURUSERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- The launcher configuration, including manually added Java versions and default username, is at `QuantumLauncher/config.json`.
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.

# Todo (in the future)
- Add ability to enable and disable mods.
- Autodownload for NeoForge, Quilt and OptiFine.
- A built in mod store using the Modrinth API.
- Integration with Omniarchive, special support for old and rare Minecraft versions (with fixes for skins/sounds)
- A local server hosting system (without port forwarding).
- macOS support.

# Licensing and Credits
A lot of this launcher's design, including the code for creating and launching the game, and installing forge, is inspired by https://github.com/alexivkin/minecraft-launcher/.

Nearly all of this launcher is licensed under the GNU General Public License v3.

However, the file in `assets/ClientInstaller.java` (required for installing forge) is licensed under the Apache 2.0 license. It's taken from the above Minecraft launcher.

# Note on Piracy
If you pirate the game, it's at your own risk. I am not responsible for any issues caused. I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.
