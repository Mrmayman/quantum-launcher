#!/usr/bin/env sh

# Check if the correct number of arguments is provided
if [ $# -ne 1 ]; then
    echo "Usage: $0 <instance_name>"
    exit 1
fi

# Find all .so files (excluding .sha1 and .git) and print their file info
find "$HOME/.config/QuantumLauncher/instances/$1/libraries/natives/" \
    -type f \( -name "*.so" -o -name "*.dll" -o -name "*.dylib" \) \
    ! -name "*.sha1" ! -path "*.git*" | while read -r FILE; do
    file "$FILE"
done
