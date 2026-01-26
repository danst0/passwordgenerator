#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FLATHUB_REPO="/home/danst/Skripte/io.github.danst0.passwordgenerator"

cd "${REPO_ROOT}"

# Check flathub repo exists
if [[ ! -d "${FLATHUB_REPO}/.git" ]]; then
    echo "Error: Flathub repo not found at ${FLATHUB_REPO}"
    exit 1
fi

# Get latest tag
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null)
if [[ -z "${LATEST_TAG}" ]]; then
    echo "Error: No tags found in repo"
    exit 1
fi

# Get commit for tag
TAG_COMMIT=$(git rev-parse "${LATEST_TAG}")

echo "Publishing ${LATEST_TAG} (${TAG_COMMIT})"

# Update flathub repo
cd "${FLATHUB_REPO}"
git fetch origin
git checkout master 2>/dev/null || git checkout main
git pull

# Copy manifest from main repo and update it
cp "${REPO_ROOT}/flathub/io.github.danst0.passwordgenerator.yml" .

# Update tag and commit in manifest
sed -i "s/tag: v[0-9.]*/tag: ${LATEST_TAG}/" io.github.danst0.passwordgenerator.yml
sed -i "s/commit: [a-f0-9]*/commit: ${TAG_COMMIT}/" io.github.danst0.passwordgenerator.yml

# Copy cargo-sources.json
cp "${REPO_ROOT}/flathub/cargo-sources.json" .

# Lint the manifest
echo "Linting manifest..."
if ! flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.danst0.passwordgenerator.yml; then
    echo "Error: Manifest failed linting"
    exit 1
fi
echo "Manifest passed linting."

# Show diff
echo ""
echo "Changes to commit:"
git diff --stat
echo ""
git diff

# Confirm
read -p "Commit and push to Flathub? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Commit and push
git add io.github.danst0.passwordgenerator.yml cargo-sources.json
git commit -m "Update to ${LATEST_TAG}"
git push

echo ""
echo "Published ${LATEST_TAG} to Flathub!"
echo "Check build status at: https://buildbot.flathub.org/#/apps/io.github.danst0.passwordgenerator"
