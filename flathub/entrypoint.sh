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

VENDOR_DIR="${REPO_ROOT}/vendor"
CARGO_DIR="${REPO_ROOT}/.cargo"
CARGO_CONFIG="${CARGO_DIR}/config.toml"
VENDOR_CREATED=false
CONFIG_CREATED=false
CONFIG_BACKUP=""

cleanup_vendor() {
    if $VENDOR_CREATED && [[ -d "${VENDOR_DIR}" ]]; then
        rm -rf "${VENDOR_DIR}"
    fi

    if [[ -n "${CONFIG_BACKUP}" && -f "${CONFIG_BACKUP}" ]]; then
        mv "${CONFIG_BACKUP}" "${CARGO_CONFIG}"
    elif $CONFIG_CREATED && [[ -f "${CARGO_CONFIG}" ]]; then
        rm -f "${CARGO_CONFIG}"
        rmdir "${CARGO_DIR}" 2>/dev/null || true
    fi
}

trap cleanup_vendor EXIT

if [[ -d "${VENDOR_DIR}" ]]; then
    echo "Removing existing vendor directory before vendoring..."
    rm -rf "${VENDOR_DIR}"
fi

mkdir -p "${CARGO_DIR}"
if [[ -f "${CARGO_CONFIG}" ]]; then
    CONFIG_BACKUP=$(mktemp)
    cp "${CARGO_CONFIG}" "${CONFIG_BACKUP}"
fi

echo "Vendoring dependencies for local Flatpak build..."
cargo vendor > "${CARGO_CONFIG}"
VENDOR_CREATED=true
CONFIG_CREATED=true

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
