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

## Publishing to Flathub

After tagging a release, publish to Flathub:

```bash
./flathub/publish.sh
```

This will:
1. Get the latest tag and commit
2. Update the Flathub repo manifest
3. Copy cargo-sources.json
4. Lint the manifest
5. Commit and push to Flathub

## Files

- `io.github.danst0.passwordgenerator.yml` - Flathub submission manifest (uses git tags + cargo-sources.json)
- `io.github.danst0.passwordgenerator.local.yml` - Local build manifest (uses local source)
- `cargo-sources.json` - Vendored cargo dependencies for offline Flathub builds
- `flatpak-cargo-generator.py` - Tool to regenerate cargo-sources.json from Cargo.lock
- `build.sh` - Local build script
- `publish.sh` - Publish to Flathub script
- `clean.sh` - Cleanup script
