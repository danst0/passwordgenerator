#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "Building Docker image..."
docker build -t passwordgenerator-builder -f "${SCRIPT_DIR}/Dockerfile" "${REPO_ROOT}"

echo "Running build container..."
# We mount the current directory to /output so the container can write the flatpak file back to us
docker run --privileged --rm -v "$(pwd):/output" passwordgenerator-builder

echo "Done! You can install the flatpak with:"
echo "flatpak install --user passwordgenerator.flatpak"

flatpak install --user passwordgenerator.flatpak