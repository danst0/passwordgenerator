# Password Generator

A simple, efficient password generator written in Rust using GTK4.

## Features

- **Grouped Output**: Generates passwords in readable groups of 5 characters (e.g., `abcde-FGHIJ-12345`).
- **Customizable Length**: Adjust the number of groups to generate passwords of desired strength.
- **Auto-Close**: Optional timer to automatically close the window after copying (security feature).
- **Clipboard Integration**: "Copy immediately" option to copy the generated password to the clipboard instantly.
- **Persistence**: Remembers your settings (groups, auto-close, copy preference) between sessions.
- **Copy Confirmation**: Highlights successful copy actions with a transient green check indicator.
- **Localized UI**: English, German, Japanese, Swedish, Spanish, Italian, and French translations load automatically based on your system locale.
- **Adaptive Theme**: The interface follows your system's light/dark preference and adjusts its accent colors accordingly.
- **Character Sets**: Toggle lowercase, uppercase, digits, and specials to create the password policy you need.

## Building and Running

### Prerequisites

- Rust (stable)
- GTK4 development libraries

### Local Build

To run the application locally using Cargo:

```bash
cargo run
```

### Flatpak Build

This project is set up to be built as a Flatpak. See [BUILD_WITH_DOCKER.md](BUILD_WITH_DOCKER.md) for instructions on building the Flatpak using Docker.

## Todo

- [x] show a green check-mark for 1.5 secs when the copy button is pressed to indicate that copying was successful
- [x] Translation (Internationalization/Localization)
- [x] Add dark mode support
- [x] Add more character set options (e.g., exclude special characters)
