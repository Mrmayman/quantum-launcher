# v0.4
It's crafting time!

## 🚀 Redesign
- Redesigned the launcher UI!
- Select instances easily with a sidebar, and enjoy the simpler navigation with tabs!
- The purple colour palette is now more vibrant and punchy
- Many other menus have been changed to look nicer
## 🛒 Mods
- Added CurseForge to the Mods store (alongside Modrinth). You can choose the backend
- Added NeoForge installer!
- Mod updating and preset importing is now nearly 2x faster!
- Getting list of versions when creating instance is now nearly 2x faster!
- The mod store now has infinite scrolling! Just scroll down to see more mods
## 🔐 Microsoft Account
- Added optional Microsoft login for those with a paid account
- Users can still continue launching the game in offline mode like earlier
## 🎮 Game
- Added option to close launcher after opening game
- Overhauled the game log viewer. There is no limit now!
- You can see the launcher debug logs by clicking the bottom bar
## 🖥️ Platform
- macOS support is now ready!
- Improved Java installer support for Windows 32 bit, Windows ARM and macOS
## 🧑‍💻 Development
- MASSIVE code cleanup and overhaul
- Redesigned the command-line experience with the "clap" library. (It's still not complete though)
- Updated "iced" to 0.13.1 from 0.12.1. Also updated many other libraries
## ⚡ Fixes
- Fixed the terminal popping up on Windows
- Fixed a bug where you couldn't disable local mods
- Fixed a JSON error when installing some mods (like debugify)
- Fixed mod management breaking from renaming instances
- Fixed a crash with 1.21.3 Fabric
- Fixed a crash with 1.21.5 Forge
- Fixed an incompatibility with wine
- Fixed many rendering bugs with the mod description viewer in the mod store
- Reduced useless log file spam in "QuantumLauncher/logs/" folder
- Reduced errors in the fabric installer
- Improved reliability of network operations, you should see "Request Error"s less now

## 💭 Coming soon
There are many things I unfortunately wasn't able to include in this release due to time constraints,
but I could add in the future such as a built in server hoster, a plugin system (scrapped for now), migration from other launchers, modpack/shaders/resource pack stores, better modding for old versions, a CLI interface, a portable binary, changing folder locations and much, much more.

---

# v0.3.1
- Quilt support is here!
- Added instance renaming!
- Omniarchive integration is here, allowing you to download old, rare versions of Minecraft. Nostalgia incoming!
- Added a brand-new style: Sky blue. Try it out in Settings -> Style!
- Added a cool ASCII art intro in the terminal

# 🛒 Mods:
- Added mod presets!
- - Share your custom mod configurations with friends
- - Download recommended mods directly from the launcher
- Now you can copy mod IDs or open the mod page in the mod store

# 💭 Misc:
- Improved command line support (try `./quantum_launcher --help` for more info)
- Added a changelog viewer
- Added a welcome screen for new users! Say hi!
- Started work on packaging (Flatpak, Deb, RPM)
- Added (experimental) support for Linux ARM64 (early alpha)
- Started work on macOS support. Hello there, you mac users!
- - If you have a mac and are willing to test the launcher, feel free to download the macOS beta from the website and try it out
- Added a confirmation dialog for uninstalling loaders
- MASSIVE codebase cleanup, optimizations and improvements (you probably won't notice it)

# ⚡ Fixes and polish:
- Launcher logs no longer have garbled text on windows
- Launcher logs are no longer delayed on windows
- Fixed a crash in some old versions of Minecraft (useLegacyMergeSort)
- Fixed a bug where OptiFine installer was broken on windows

---

# v0.3
QuantumLauncher v0.3 introduces a powerful mod manager, OptiFine support, and a built-in mod store for downloading mods seamlessly. Plus, we’ve made massive performance improvements and fixed critical issues with older Minecraft versions.

## 🛒 Mods
- Select, enable, disable and delete your mods with the new mod manager!
- Added OptiFine support! (Mods->Install OptiFine)
### 🛍️ Store
- Added a built-in mod store, integrated with modrinth.com (Mods->Download Mods)
- You can now search, view and download mods
- *note: this is experimental and may be buggy*

## ⚡ Optimization
- *DRASTICALLY* improved download times for creating an instance
- Asset files are shared between instances, saving a lot of storage

## ⛏️ Old Minecraft
- Fixed sound being broken in old versions (1.6.4 and below) due to incorrect asset downloading
- Betacraft proxy is now used for older versions, resulting in correct sounds and skins

## ⚙️ Settings
- Added launcher themes (light/dark mode) and styles (brown/purple color scheme)
- The launcher settings menu contains links to our GitHub, website and discord.
- You can now edit Java and Game arguments in instance settings.
- Added a debugging option to disable log output (to see advanced crash messages correctly in the terminal window). Use this if your instance crashed without any logs. Edit Instance -> Enable logging (disable it)

## 📟 Command-Line
- `--command` argument to not make it pop up a window (headless)
- `--list-instances` flag to print a list of instances (name, version, loader)
- More headless functionality coming soon

## 💭 Other
- Fixed a crash with modern versions of minecraft and fabric/forge
- Added a search bar for create instance version list
- Cleaned up and improved the menus for Forge and Fabric installers

---

# v0.2
## Features
### Forge
- Added a Forge installer (and uninstaller too)
- Select your instance, go to `Mods` -> `Install Forge` and you're done!
- Note: Only release 1.6.4 and above are currently supported.
### Logs
- Now you can view game logs!
- When the game launches, the logs will be available on the right side.
- Press `Copy Logs` to copy the logs (if you face any issues you can send it)
- Only a section of the log is shown for performance reasons, but you can get the whole thing by pressing the `Copy Logs` button.
### Misc
- There is now an update detector that looks for launcher updates and asks you if you want to install it. If you accept, it will automatically update the launcher and create a backup for you.
- The UI design has been changed to be cleaner.
- Added a progress bar for Fabric installer.
- Added an uninstaller for Fabric.
- Now there are basic command line options: `--version` and `--help`. I am planning to add a headless command line mode in the future.
- Now the game's current directory will be the `.minecraft` folder. This fixes random files popping up near the launcher executable.
- Now the debug output of the launcher text incorporates colored text and dynamic progress bars.

Download the zip as per your OS. Extract it and run the executable (or if you download source code, just compile it) and you have your launcher!
If anything is broken, please message me on discord at @Mrmayman (server link in readme)
This is in early alpha, some stuff might be broken.

---

# v0.1
The first release of QuantumLauncher. Very buggy and janky.

## Features:
- Make separate instances of Minecraft with one click.
- Auto-installs Java.
- Built in Fabric installer.

## Doesn't have:
- Log viewer (added in v0.2)
- Forge/OptiFine installer (added in v0.2)
- Skin/Sound fixes for old versions (added in v0.3)
- Archived old versions of Minecraft from OmniArchive.

## Notes:
Download the zip as per your OS. Extract it and run the executable (or if you download source code, just compile it) and you have your launcher!
If anything is broken, please message me on discord at @mrmayman
This launcher has been in development for quite a while, I just thought of making the first release now.
