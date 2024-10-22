# QuantumLauncher
A minimalistic Minecraft launcher for Windows and Linux written in *Rust* with the *iced* framework.

![Preview](quantum_launcher.png)

- Website (+ Download): https://mrmayman.github.io/quantumlauncher

- Discord: https://discord.gg/bWqRaSXar5

# Features
- Lightweight and responsive. No more minute-long loading screens, no more laggy buttons
- Automatically manages Java installations
## Instances:
- Isolate your different game versions with instances!
- Create or delete an Instance easily.
## Modding:
- Install fabric and forge with one click.
- Build in mod store to download your favorite mods (experimental feature, will be released in next update)

# Downloads and Building
- You can download the stable version from the website linked above, or from the *Releases*
- Or, you can compile the launcher to get the latest experimental version (with potentially broken and untested features).
- To compile the launcher:
```
git clone https://github.com/Mrmayman/quantum-launcher.git
cd quantum-launcher
cargo run --release
```
You can omit the `--release` flag for faster compile times, but *slightly* worse performance and MUCH larger build file size.

# File Locations
- On *Windows*, the launcher files are at `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.

# To-do (in the future)
- Add ability to enable and disable mods.
- Autodownload for NeoForge, Quilt and OptiFine.
- A built in mod store using the Modrinth API (available in the experimental version, clone the repo and compile it. Warning: Not stable or fully functional)
- Integration with Omniarchive, special support for old and rare Minecraft versions (with fixes for skins/sounds)
- A local server hosting system (without port forwarding).
- macOS support.

# Licensing and Credits
A lot of this launcher's design, including the code for creating and launching the game, and installing forge, is inspired by https://github.com/alexivkin/minecraft-launcher/.

Nearly all of this launcher is licensed under the GNU General Public License v3.

- However, the file in `assets/ClientInstaller.java` (required for installing forge) is licensed under the Apache 2.0 license. It's taken from the above Minecraft launcher.
- Additionally the `.github/workflows/build.yml` file is licensed under Apache 2.0.

# Note on Piracy
If you pirate the game, it's at your own risk. I am not responsible for any issues caused. I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.
