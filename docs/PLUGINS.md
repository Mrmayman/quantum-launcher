# Lua Plugin System (scrapped)

I initially made a lua-based plugin system for QuantumLauncher but ended up scrapping it due to many reasons:
- Code complexity and bloat
- Compile times, binary size
- Lack of use; In this small community who would be making plugins? Besides if anyone wants a feature, I can just add it for them.

## Where to find it

Commits `867e539f0495f8ef11bef38ae7b5ba1cad821939` (6 February 2025) to `16e02b1e36a736fadb3214b84de908eb21635a55` (4 April 2025) have the removed plugin system, if anyone's interested.
The code is in `crates/ql_plugins` as well as a commented-out snippet in the `main` function in `quantum_launcher/src/main.rs`

If anyone's interested, feel free to create a fork and fix this messy implementation. I personally am not interested in developing a plugin system though.

## Implementation

The plugin system used the `mlua` library with Lua 5.1.

Here is the old, scrapped documentation.

---

# QuantumLauncher Plugins Guide

Plugins extend the functionality of QuantumLauncher.
This document attempts to guide you into making one.

# Getting started

Create a plugin folder *somewhere*. If you want to propose
bundling it into the launcher then clone this repository
and create the folder here, in `plugins/` and make a pull request.

A centralized plugin store may come in the future.

Let's call this plugin folder `installer_optifine`. Create an `index.json`
file inside it. Here is an example of what to fill in:

TODO

# Functions

## Logging

### `qlLogInfo(...)`
Logs something as an `[info]` message.

### `qlLogError(...)`
Logs something as an `[error]` message.

### `qlLogPt(...)` or `print(...)`
Logs something as a bullet point message, ie.
less important than an info or error message.

## Java

### `qlJavaExec(name: String, version: i32, progress: Option<LuaGenericProgress>, [args], current_dir: Option<String>)`
Executes any specified Java binary with the specified Java version.
For example:

```lua
-- Runs the java compiler (javac) of Java 8 with the "-version" argument
qlJavaExec("javac", 8, nil, {"-version"})
```

This automatically installs Java if not present. You can optionally supply a progress hook
(`LuaGenericProgress` which wraps around `std::sync::Arc<std::sync::mpsc::Sender<GenericProgress>>`)

This requires the `Java` permission as it can be **very** dangerous when untrusted.

## File Picking

### `qlPickFile(window_title: String, filters: [String], filter_name: String) -> String`
Prompts the user to pick a file. A file browser window will open,
the user will select a file and the file contents will be returned.

If you want to filter for specific extensions use the filters.

Example:
```lua
-- Prompts the user to select a jar file
local file = qlPickFile("Select a jar file", {"jar"}, "Jar File")
```

## Requests

### `qlDownload(url: String, user_agent: bool) -> String`
Downloads the file at `url` and returns it as a lua string.

If `user_agent` is true, this will use the quantumlauncher user agent.
If it's false then this won't use any user agent.
