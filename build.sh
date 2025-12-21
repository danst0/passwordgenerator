#!/bin/bash
set -e

echo "Building Docker image..."
docker build -t passwordgenerator-builder .

echo "Running build container..."
# We mount the current directory to /output so the container can write the flatpak file back to us
docker run --privileged --rm -v "$(pwd):/output" passwordgenerator-builder

echo "Done! You can install the flatpak with:"
echo "flatpak install --user passwordgenerator.flatpak"

flatpak install --user passwordgenerator.flatpak