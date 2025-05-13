#!/bin/bash

# Function to process a single jar file
process_jar() {
    local jar_file="$1"

    # Extract the list of files in the jar and filter for *.so files
    local so_files
    so_files=$(jar tf "$jar_file" | grep -E '\.(so|dll|dylib|jnilib)$')

    # Only print if there are any .so files
    if [[ -n "$so_files" ]]; then
        echo "$jar_file:"
        echo "$so_files" | while read -r so_file; do
            echo "  /$so_file"
        done
    fi
}

# Main function
process_dir() {
    local dir="$1"

    # Check if the directory exists
    if [[ ! -d "$dir" ]]; then
        echo "Error: Directory '$dir' does not exist."
        exit 1
    fi

    # Find all jar files recursively in the directory
    find "$dir" -type f -name '*.jar' | while read -r jar_file; do
        process_jar "$jar_file"
    done
}

# Check if a directory path is provided
if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <path_to_directory>"
    exit 1
fi

# Process the given directory
process_dir "$HOME/.config/QuantumLauncher/instances/$1/libraries"
