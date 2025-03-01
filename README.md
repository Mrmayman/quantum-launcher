<p align="center">

# <img src="assets/icon/ql_logo.png" style="height: 1.4em; vertical-align: middle;" /> QuantumLauncher
## [Website](https://mrmayman.github.io/quantumlauncher) | [Discord](https://discord.gg/bWqRaSXar5)

A minimalistic Minecraft launcher for Windows and Linux (and soon-to-be macOS).

<img src="quantum_launcher.png" width="100%" />

</p>

QuantumLauncher is written in *Rust* with the *iced* framework,
offering a lightweight and responsive experience.
It is designed to be simple and easy to use, with a focus on performance and features.

<p align="center">

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

</p>

You can download the stable version from the website linked above, or from the *Releases* button

Or, you can compile the launcher to get the latest experimental version (with potentially broken and untested features).
To compile the launcher:

```
git clone https://github.com/Mrmayman/quantum-launcher.git
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
- PrismLauncher is a great launcher overall, but it does not support
  offline accounts. Same for MultiMC.
- Legacy Launcher isn't as feature rich as this
- TLauncher is *suspected* to be malware

<p align="center">

# File Locations

</p>

- On *Windows*, the launcher files are at `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`.
- You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter.
- On *Linux*, the launcher files are at `~/.config/QuantumLauncher/`. (`~` refers to your home directory).
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`.
- Launcher logs are located at `QuantumLauncher/logs/`.

<br>

<p align="center">

# To-do (in the future)

</p>

## Core
- [x] Instance creation
- [x] Instance launching
- [x] Instance deletion
- [x] Instance renaming
- [x] Java/Game args editing
- [x] Memory allocation editing
- [x] Optional Microsoft login
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
## Instances
- [ ] Import MultiMC instance
- [ ] Migrate from other launchers (file locations)
- [ ] Package QuantumLauncher instance
## Platforms
- [x] Windows x86_64
- [x] Linux x86_64
- [x] Linux Aarch64 (WIP)
- [ ] macOS (WIP)
- [ ] Windows i686 (WIP)
- [ ] Linux i686 (WIP)
- [ ] Windows Aarch64 (WIP)
- [ ] Android (distant future)
## Misc
- [x] Integration with Omniarchive, old version support
- [ ] A local server hosting system (without port forwarding, using proxy tunneling) (WIP)
- [ ] Plugin system (with lua) (WIP)

<p align="center">

# Contributing

</p>

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

There's a more in-depth guide on the codebase in [main.rs](quantum_launcher/src/main.rs) at the top.

## Contributors
- [Mrmayman](https://github.com/Mrmayman) (lead developer)
- [apicalshark](https://github.com/apicalshark) (github CI, packaging, distribution)
- Aurlt (@exsclt_35289 on Discord) (icon design)

<p align="center">

# Licensing and Credits

</p>

A lot of this launcher's design, including the code for creating and launching the game,
and installing forge, is inspired by https://github.com/alexivkin/minecraft-launcher/.

Nearly all of this launcher is licensed under the **GNU General Public License v3**,
however there are a few exceptions (such as github actions and assets).
Visit [the assets README](assets/README.md) for more information.

<p align="center">

# Note on Piracy

If you pirate the game, it's at your own risk. I am not responsible for any issues caused.
I recommend that you buy the game, but if you don't have the means, feel free to use this launcher.
If anyone has any issues/complaints, just open an issue in the repo.

</p>
