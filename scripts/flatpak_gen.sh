#!/usr/bin/env sh
flatpak-cargo-generator.py -o flathub-sources.json Cargo.lock

# The `flatpak-cargo-generator.py` must be in your `$PATH` (or otherwise just copy-paste the above line
# with the path to the python script).
# You can download the required python script from the following URL
# and put it in your $PATH. Make sure to `chmod +x path/to/flatpak-cargo-generator.py` on macOS/linux.

# https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
