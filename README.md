# <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/icon/ql_logo.png" style="height: 1.4em; vertical-align: middle;" /> QuantumLauncher
## [Website](https://mrmayman.github.io/quantumlauncher) | [Discord](https://discord.gg/bWqRaSXar5) | [Changelogs](https://github.com/Mrmayman/quantumlauncher/raw/main/CHANGELOGS.md)

A minimalistic Minecraft launcher for Windows, macOS and Linux.

![Quantum Launcher running a Minecraft Instance](https://github.com/Mrmayman/quantumlauncher/raw/main/quantum_launcher.png)

QuantumLauncher is written in *Rust* with the *iced* framework,
offering a lightweight and responsive experience.
It is designed to be simple and easy to use, with a focus on performance and features.

# Features

## Lightweight and responsive

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/lightweight.png)

## Install fabric, forge or optifine with ease

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/install_loader.png)

## Build in mod store to download your favorite mods

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/mod_store.png)

## Isolate your different game versions with instances!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/new.png)

## Full support for old minecraft versions, integrated with Omniarchive. Includes skin and sound fixes!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/old_mc.png)

## Neatly package your mods into presets, and share it with your friends!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/presets.png)

## Built in themes!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/themes.png)
<br><br>

# Downloads and Building

You can download the stable version from the website linked above, or from the *Releases* button

Or, you can compile the launcher to get the latest experimental version (with potentially broken and untested features).
To compile the launcher:

```sh
git clone https://github.com/Mrmayman/quantumlauncher.git
cd quantum-launcher
cargo run --release
```
You can omit the `--release` flag for faster compile times, but *slightly* worse performance and MUCH larger build file size.

# Why QuantumLauncher?
- QuantumLauncher provides a feature rich, flexible, simple
  and lightweight experience with plenty of modding features.

What about the others? Well...

- The official Minecraft launcher is slow, unstable, buggy and frustrating to use,
  with barely any modding features.
- Prism Launcher is a great launcher overall, but it does not support
  offline accounts. Same for MultiMC.
- Legacy Launcher isn't as feature rich as this
- TLauncher is *suspected* to be malware

# File Locations

- On *Windows*, the launcher files are at `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.
- Launcher logs are located at `QuantumLauncher/logs/`.

<br>

# To-do (in the future)

(note: WIP means work-in-progress)

## Core
- [x] Instance creation, deletion, renaming, launching
- [x] Java/Game args editing
- [x] Memory allocation editing
- [x] Optional Microsoft login
- [x] Integration with Omniarchive, old version support
- [ ] Full controller, keyboard-navigation support in UI

## Mods
### Loaders
- [x] Fabric
- [x] Forge
- [x] Optifine
- [x] Quilt
- [x] Neoforge
- [ ] OptiForge
- [ ] OptiFabric
- [x] Jar Mods
### Sources
- [x] Modrinth mods
- [x] Curseforge mods
- [x] Modrinth modpacks
- [x] Curseforge modpacks
### Features
- [x] Mod store
- [x] Mod presets (packaging mods)
- [x] Mod updater
- [ ] Make mod updater incrementally load in (optimization)
- [ ] UI/UX overhaul of preset system
- [ ] Category Filters in Mod store

## Instances
- [ ] Import MultiMC/PrismLauncher instances
- [ ] Migrate from other launchers
- [ ] Package QuantumLauncher instances (in progress by @sreehari425)
- [ ] Upgrading instances to a newer Minecraft version

## Servers (disabled in GUI but can be enabled)
- [x] Ability to create, delete and run Minecraft servers
- [x] Editing basic server settings (RAM, Java, Args)
- [ ] Editing `server.properties`
- [ ] Editing any NBT config file
- [ ] Plugin store
- [ ] [playit.gg](https://playit.gg) integration
### Loaders
- [x] Paper
- [ ] Spigot
- [ ] Bukkit
- [ ] Bungeecoord
- [ ] The stuff from [MODS+PLUGINS.md](https://github.com/LeStegii/server-software/blob/master/java/MODS+PLUGINS.md)

## Platforms
- [x] Windows x86_64
- [x] Linux x86_64
- [ ] macOS x86_64 (WIP)
- [ ] Windows Aarch64 (WIP)
- [x] Linux Aarch64 (Almost ready)
- [x] macOS Aarch64 (Almost ready)
- [ ] Windows i686 (WIP)
- [ ] Linux i686 (WIP)
- [ ] FreeBSD
- [ ] Haiku

## Command-Line interface
- [x] List installed instances `list-instances`, `-l`
- [x] List versions available for download `list-available-versions`, `-a`
- [ ] Create instance from CLI
- [ ] Launch instance from CLI
- [ ] Install loaders from CLI
- [ ] Mod installation features from CLI
- [ ] Preset, modpack features from CLI

# MSRV (Minimum Supported Rust Version)

- The exact MSRV is unknown (feel free to find out for yourselves).
- However, at least Rust 1.78.0 is required.

# Contributing/Contributors
For more info, see [CONTRIBUTING.md](https://github.com/Mrmayman/quantumlauncher/tree/main/docs/CONTRIBUTING.md)

# Licensing and Credits

A lot of this launcher's design, including the code for creating and launching the game,
and installing forge, is inspired by <https://github.com/alexivkin/minecraft-launcher/>.

Nearly all of this launcher is licensed under the **GNU General Public License v3**,
however there are a few exceptions (such as github actions and assets).
Visit [the assets README](assets/README.md) for more information.

# Note on Piracy

If you pirate the game, it's at your own risk. I am not responsible for any issues caused.
I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.
