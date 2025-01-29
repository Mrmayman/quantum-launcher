# These are some bash scripts that I use to automate some tasks.
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
