#!/bin/bash
set -e

REPO_ROOT="/app"
MANIFEST="flathub/io.github.danst0.passwordgenerator.yml"

cd "${REPO_ROOT}"

echo "Starting build process..."

# Vendor dependencies
# This downloads all Rust dependencies and configures cargo to use them offline
if [ ! -d "vendor" ]; then
    echo "Vendoring dependencies..."
    mkdir -p .cargo
    cargo vendor > .cargo/config.toml
    # Append to existing config if needed, but here we overwrite or create
    # If you have custom config, you might want to merge.
else
    echo "Vendor directory already exists."
fi

# Clean previous builds
rm -rf build-dir repo

echo "Building Flatpak..."
# --sandbox is default, but inside docker we might need to be careful.
# flatpak-builder uses bubblewrap.
flatpak-builder --repo=repo --force-clean build-dir "${MANIFEST}"

echo "Creating Bundle..."
flatpak build-bundle repo passwordgenerator.flatpak io.github.danst0.passwordgenerator

echo "Build complete: passwordgenerator.flatpak"

# Copy to output if mounted
if [ -d "/output" ]; then
    echo "Copying to /output..."
    cp passwordgenerator.flatpak /output/
    chown $(stat -c '%u:%g' /output) /output/passwordgenerator.flatpak || true
    echo "Done."
else
    echo "No /output directory found. The flatpak is in /app/passwordgenerator.flatpak inside the container."
fi
