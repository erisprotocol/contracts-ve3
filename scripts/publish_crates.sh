#!/usr/bin/env bash

set -e
set -o pipefail

# Ensure the user is logged in to crates.io
if ! cargo search invalid-crate-name &> /dev/null; then
    echo "Please log in to crates.io using 'cargo login'."
    exit 1
fi

# Directory containing the crates (adjust the path if needed)
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATES_DIR="$SCRIPTS_DIR/../packages"

if [ ! -d "$CRATES_DIR" ]; then
    echo "Directory $CRATES_DIR does not exist."
    exit 1
fi

# Publish each crate in the directory
for crate in "$CRATES_DIR"/*; do
    if [ -d "$crate" ]; then
        echo "Publishing crate: $crate"
        cd "$crate"
        # Check if Cargo.toml exists in the directory
        if [ -f "Cargo.toml" ]; then
            # Package the crate
            cargo package
            # Publish the crate
            cargo publish
        else
            echo "No Cargo.toml found in $crate. Skipping."
        fi
        cd - > /dev/null
    else
        echo "$crate is not a directory. Skipping."
    fi
done

echo "All crates under $CRATES_DIR have been published."
