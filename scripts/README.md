# These are shell scripts that I use to automate some tasks.
TODO: Support windows (these only support linux/macOS currently)

## `clippy-pedantic.sh`
This script is used to run clippy with pedantic level (very strict).

## `list_downloaded_natives.sh <instance_name>`
This script is used to list all the native libraries
downloaded by the launcher, and their architecture.

## `list_so_files.sh <instance_name>`
This script is used to list all the `.so` files that *could* be
extracted and installed. Unlike the other script, this one lists
all the possible `.so` files, not just the ones that are actually installed.

## `arm64_build.sh`
A script to build for Arm 64 using `cross`

## `flatpak_gen.sh`
A script to generate a flatpak dependency list based on `Cargo.lock`.
This is important when publishing to flathub.

The `flatpak-cargo-generator.py` must be in your `$PATH` (or otherwise just copy-paste this shell script
with the path to the python script).
You can download the required python script from <https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py>
and put it in your path. Make sure to `chmod +x path/to/flatpak-cargo-generator.py` on macOS/linux.
