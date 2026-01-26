#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MANIFEST="${SCRIPT_DIR}/io.github.danst0.passwordgenerator.yml"

cd "${REPO_ROOT}"

# Check for required tools
for cmd in flatpak flatpak-builder cargo; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "Error: $cmd is not installed"
        exit 1
    fi
done

# Ensure required Flatpak runtimes are installed
echo "Checking Flatpak runtimes..."
if ! flatpak info org.gnome.Platform//49 &> /dev/null; then
    echo "Installing org.gnome.Platform//49..."
    flatpak install -y flathub org.gnome.Platform//49
fi
if ! flatpak info org.gnome.Sdk//49 &> /dev/null; then
    echo "Installing org.gnome.Sdk//49..."
    flatpak install -y flathub org.gnome.Sdk//49
fi
if ! flatpak info org.freedesktop.Sdk.Extension.rust-stable//25.08 &> /dev/null; then
    echo "Installing org.freedesktop.Sdk.Extension.rust-stable//25.08..."
    flatpak install -y flathub org.freedesktop.Sdk.Extension.rust-stable//25.08
fi

# Vendor dependencies
VENDOR_DIR="${REPO_ROOT}/vendor"
CARGO_DIR="${REPO_ROOT}/.cargo"
CARGO_CONFIG="${CARGO_DIR}/config.toml"
VENDOR_CREATED=false
CONFIG_CREATED=false
CONFIG_BACKUP=""

cleanup_vendor() {
    if $VENDOR_CREATED && [[ -d "${VENDOR_DIR}" ]]; then
        echo "Cleaning up vendor directory..."
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
    echo "Removing existing vendor directory..."
    rm -rf "${VENDOR_DIR}"
fi

mkdir -p "${CARGO_DIR}"
if [[ -f "${CARGO_CONFIG}" ]]; then
    CONFIG_BACKUP=$(mktemp)
    cp "${CARGO_CONFIG}" "${CONFIG_BACKUP}"
fi

echo "Vendoring dependencies..."
cargo vendor > "${CARGO_CONFIG}"
VENDOR_CREATED=true
CONFIG_CREATED=true

# Clean previous builds
rm -rf "${REPO_ROOT}/build-dir" "${REPO_ROOT}/repo"

echo "Building Flatpak..."
flatpak-builder --repo=repo --force-clean build-dir "${MANIFEST}"

echo "Creating bundle..."
flatpak build-bundle repo passwordgenerator.flatpak io.github.danst0.passwordgenerator

echo ""
echo "Build complete: passwordgenerator.flatpak"
echo ""

# Ask to install
read -p "Install the Flatpak now? [Y/n] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    flatpak install --user -y passwordgenerator.flatpak
    echo "Installed! Run with: flatpak run io.github.danst0.passwordgenerator"
fi
