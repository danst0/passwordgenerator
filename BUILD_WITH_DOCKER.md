# Docker Flatpak Build

This directory contains a Dockerfile to build the Flatpak for `passwordgenerator`.

## Prerequisites

- Docker installed and running.

## Building the Image

Build the Docker image containing the SDK and tools:

```bash
docker build -t passwordgenerator-builder .
```

## Running the Build

Run the container to build the Flatpak. You need to run with `--privileged` because `flatpak-builder` uses bubblewrap (sandboxing) which requires privileges inside a container.

Mount the current directory to `/output` to receive the generated `.flatpak` file.

```bash
docker run --privileged --rm -v $(pwd):/output passwordgenerator-builder
```

After the command finishes, you should see `passwordgenerator.flatpak` in your current directory.

## Installing the Flatpak

```bash
flatpak install --user passwordgenerator.flatpak
```

## Running the App

```bash
flatpak run io.github.danst0.passwordgenerator
```
