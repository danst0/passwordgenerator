# Password Generator – Flathub Packaging

This directory contains the Flatpak manifest and build scripts for Password Generator.

## Prerequisites

- `flatpak` and `flatpak-builder` installed
- `cargo` (Rust toolchain)
- Flathub remote configured: `flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo`

## Building

Run the build script from the repository root or flathub directory:

```bash
./flathub/build.sh
```

The script will:
1. Check for required tools
2. Install required Flatpak runtimes if missing (org.gnome.Platform//49, org.gnome.Sdk//49, rust-stable extension)
3. Vendor Cargo dependencies
4. Build the Flatpak using flatpak-builder
5. Create a `.flatpak` bundle
6. Optionally install it

## Manual Installation

If you skipped the install prompt:

```bash
flatpak install --user passwordgenerator.flatpak
```

## Running

```bash
flatpak run io.github.danst0.passwordgenerator
```

## Cleaning Build Artifacts

```bash
./flathub/clean.sh
```

## Files

- `io.github.danst0.passwordgenerator.yml` - Local build manifest (uses local source)
- `io.github.danst0.passwordgenerator.flathub.yml` - Flathub submission manifest (uses git tags)
- `build.sh` - Build script
- `clean.sh` - Cleanup script
