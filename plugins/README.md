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
