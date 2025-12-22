#!/bin/bash
set -e

if [[ -d "/workspace" ]]; then
    REPO_ROOT="/workspace"
else
    REPO_ROOT="/app"
fi
MANIFEST="flathub/io.github.danst0.passwordgenerator.yml"

cd "${REPO_ROOT}"

echo "Starting build process..."

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
