# Docker Flatpak Build

This directory contains a Dockerfile to build the Flatpak for `passwordgenerator`.

## Prerequisites

- Docker installed and running.

## Building the Image

Build the Docker image containing the SDK and tools (run from the repository root):

```bash
docker build -t passwordgenerator-builder -f flathub/Dockerfile .
```

## Running the Build

Use the helper script to run the container and collect the resulting bundle:

```bash
./flathub/build.sh
```

The script mounts the repository into the container, triggers the Flatpak build, and writes `passwordgenerator.flatpak` back to the project root.

## Installing the Flatpak

```bash
flatpak install --user passwordgenerator.flatpak
```

## Running the App

```bash
flatpak run io.github.danst0.passwordgenerator
```
