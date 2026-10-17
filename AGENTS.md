# AGENTS.md

This file provides guidance to coding agents (Claude Code, Codex, …) when working with code in this repository.

## Project Overview

Password Generator is a GTK4-based GNOME desktop application written in Rust. It generates secure passwords with customizable character sets and displays them in grouped segments (e.g., `abcde-FGHIJ-12345`).

**Application ID**: `io.github.danst0.passwordgenerator`

## Build Commands

```bash
cargo run                 # Run the application
cargo build --release     # Build optimized binary
cargo check               # Type check without building
cargo test                # Run tests
```

### Flatpak Build

```bash
./flathub/build.sh
```

Requires `flatpak`, `flatpak-builder`, and `cargo`. The script auto-installs required runtimes.

## Architecture

Almost all logic lives in `src/main.rs` (~1800 lines). `src/wayland_selection.rs` sets the clipboard on GTK's own Wayland connection with the keyboard-enter serial, because GTK's clipboard only uses key/button-press serials and is ignored by mutter before the first user input (needed for copy-on-startup).

### Key Data Structures

- **`AppSettings`** - Serializable user preferences persisted to `~/.config/passwordgenerator/settings.json`
- **`GenerationOptions`** - Character set toggles (lowercase, uppercase, digits, special)
- **`I18nStrings`** - Localization strings with factory pattern for 8 supported languages

### Password Generation

`generate_password(groups, options, use_default_strategy)` is the core function:
- **Default strategy**: Guarantees at least one uppercase, digit, and special character
- **Custom strategy**: Uses only the selected character sets
- Passwords are grouped in 5-character segments

### State Management Patterns

- `Rc<RefCell<T>>` for shared mutable state
- `Rc<Cell<T>>` for non-Drop types
- Weak references (`downgrade()`) to break circular references in GTK signal closures

### Localization

Translations are inline in `main.rs` using a factory pattern:
- `strings_en()`, `strings_de()`, `strings_ja()`, etc.
- System locale detected via `glib::language_names()`
- Add new languages by implementing a new `strings_xx()` function and adding to `localized_strings()` match

## Key Files

- `src/main.rs` - Application code
- `src/wayland_selection.rs` - Wayland clipboard write for copy-on-startup
- `data/*.desktop` - Desktop entry
- `data/*.metainfo.xml` - AppStream metadata (update for releases)
- `flathub/*.yml` - Flatpak manifests

## Release Process

1. Update version in `Cargo.toml`
2. Update version and release notes in `data/io.github.danst0.passwordgenerator.metainfo.xml`
3. Tag release with `v{version}`
