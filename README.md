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

# Contributing

There are many ways you can help me out! I'm open to any contribution:

## If you don't know how to code, you can:
- Find and report bugs or issues
- Give feedback about how this launcher could be improved
- Fix any typos or mistakes in anything (english isn't my first language)
- Most importantly, share this launcher with your friends!

## If you know how to code, you can:
- Well... write code. Add stuff. Don't worry about "quality"
  or fancy terms like that. This ain't the linux kernel, I'm here with you!
- Write documentation. See a public function, module, struct, enum, whatever
  that could help with some `///` doc comment? Go ahead!
- Contribute to the website (repo: <https://github.com/Mrmayman/Mrmayman.github.io>)
- Work on CI (github actions)

# Codebase
This section will mainly focus on what the
codebase is like for any potential contributors.

## Crate Structure
- `quantum_launcher` - The GUI frontend
- `ql_instances` - Instance management, updating and launching
- `ql_mod_manager` - Mod management and installation
- `ql_servers` - A self-hosted server management system (incomplete)
- `ql_packager` - Code related to packaging/importing instances
### Core components
- `ql_core` - Core utilities and shared code
- `ql_reqwest` - A shim (wrapper) around the [reqwest](https://github.com/seanmonstar/reqwest) library, that automatically deals with platform-specific features in the Cargo.toml.
### Specific-purpose "libraries"
- `ql_java_handler` - A library to auto-install and provide java runtimes

## Brief Overview of the codebase
The architecture of the launcher is based on the
Model-View-Controller pattern (AKA the thing used in iced).

- The `Launcher` struct is the main controller of the application.
- `view()` renders the app's view based on the current state.
- `update()` processes messages and updates the state accordingly.
- The `state::State` enum determines which menu is currently open.

So it's a back-and-forth between `Message`s coming from interaction,
and code to deal with the messages in `update()`.

## Comments
I tend to be loose, for better or for worse,
when it comes to using comments.
Have something complicated-looking that could
be better explained? Add comments. Clippy bugging you
about not documenting something? Add doc comments.

**The only rule of thumb is: Do it well or don't do it**.

Half-baked useless comments are worse than no comments
(yes I'm guilty of this sometimes).

Heck, feel free to make it informal if that seems better.
(maybe add a `WTF: ` tag so people can search for it for fun).

Btw, if you have any questions, feel free to ask me on discord!

# Contributors
- [Mrmayman](https://github.com/Mrmayman) (lead developer)
- [apicalshark](https://github.com/apicalshark) (github CI, packaging, distribution)
- Aurlt (@exsclt_35289 on Discord) (icon design)

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
