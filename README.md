<div style="text-align: center;">

# <img src="assets/icon/ql_logo.png" style="height: 1.4em; vertical-align: middle;" /> QuantumLauncher
## [Website](https://mrmayman.github.io/quantumlauncher) | [Discord](https://discord.gg/bWqRaSXar5)

A minimalistic Minecraft launcher for Windows and Linux (and soon-to-be macOS).

<img src="quantum_launcher.png" width="100%" />

</div>

QuantumLauncher is written in *Rust* with the *iced* framework,
offering a lightweight and responsive experience.
It is designed to be simple and easy to use, with a focus on performance and features.

<div style="text-align: center;">

# Features

## Lightweight and responsive

<img src="assets/screenshots/lightweight.png" width="70%" />

## Install fabric, forge or optifine with ease

<img src="assets/screenshots/install_loader.png" width="70%" />

## Build in mod store to download your favorite mods

<img src="assets/screenshots/mod_store.png" width="70%" />

## Isolate your different game versions with instances!

<img src="assets/screenshots/new.png" width="70%" />

## Full support for old minecraft versions, integrated with Omniarchive. Includes skin and sound fixes!

<img src="assets/screenshots/old_mc.png" width="70%" />

## Neatly package your mods into presets, and share it with your friends!

<img src="assets/screenshots/presets.png" width="70%" />

## Built in themes!

<img src="assets/screenshots/themes.png" width="70%" />
<br><br>

# Downloads and Building

</div>

You can download the stable version from the website linked above, or from the *Releases* button

Or, you can compile the launcher to get the latest experimental version (with potentially broken and untested features).
To compile the launcher:

```
git clone https://github.com/Mrmayman/quantum-launcher.git
cd quantum-launcher
cargo run --release
```
You can omit the `--release` flag for faster compile times, but *slightly* worse performance and MUCH larger build file size.

<div style="text-align: center;">

# File Locations

</div>

- On *Windows*, the launcher files are at `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.
- Launcher logs are located at `QuantumLauncher/logs/`.

<br>

<div style="text-align: center;">

# To-do (in the future)

</div>

## Core
- [x] Instance creation
- [x] Instance launching
- [x] Instance deletion
- [x] Instance renaming
- [x] Java/Game args editing
- [x] Memory allocation editing
- [ ] Optional Microsoft login
## Mods
### Loaders
- [x] Fabric
- [x] Forge
- [x] Optifine
- [x] Quilt
- [ ] Neoforge
- [ ] OptiForge
- [ ] OptiFabric
- [ ] Jar Mods
### Sources
- [x] Modrinth mods
- [ ] Curseforge mods
- [ ] Modrinth modpacks
- [ ] Curseforge modpacks
### Features
- [x] Mod store
- [x] Mod updater
- [x] Mod presets
## Platforms
- [x] Windows x86_64
- [x] Linux x86_64
- [ ] Linux Aarch64 (WIP)
- [ ] macOS (WIP)
- [ ] Android (distant future)
## Misc
- [x] Integration with Omniarchive, old version support
- [ ] A local server hosting system (without port forwarding, using proxy tunneling) (WIP)
- [ ] Plugin system (with lua) (WIP)

<div style="text-align: center;">

# Licensing and Credits

</div>

A lot of this launcher's design, including the code for creating and launching the game, and installing forge, is inspired by https://github.com/alexivkin/minecraft-launcher/.

Nearly all of this launcher is licensed under the GNU General Public License v3,
however there are a few exceptions (such as github actions and assets).
Visit [the assets README](assets/README.md) for more information.

<div style="text-align: center;">

# Note on Piracy

If you pirate the game, it's at your own risk. I am not responsible for any issues caused.
I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.

</div>
