#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FLATHUB_REPO="/home/danst/Skripte/io.github.danst0.passwordgenerator"
MANIFEST_FILE="io.github.danst0.passwordgenerator.yml"

cd "${REPO_ROOT}"

# Check flathub repo exists
if [[ ! -d "${FLATHUB_REPO}/.git" ]]; then
    echo "Error: Flathub repo not found at ${FLATHUB_REPO}"
    exit 1
fi

# Check for gh CLI
if ! command -v gh &> /dev/null; then
    echo "Error: gh (GitHub CLI) is required"
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ -z "${VERSION}" ]]; then
    echo "Error: Could not read version from Cargo.toml"
    exit 1
fi

TAG="v${VERSION}"
echo "Version from Cargo.toml: ${VERSION}"
echo "Expected tag: ${TAG}"

# Check if tag exists
if ! git rev-parse "${TAG}" &>/dev/null; then
    echo ""
    echo "Tag ${TAG} does not exist."
    read -p "Create tag ${TAG} now? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag "${TAG}"
        git push origin "${TAG}"
        echo "Created and pushed tag ${TAG}"
    else
        echo "Aborted. Please create the tag first:"
        echo "  git tag ${TAG} && git push origin ${TAG}"
        exit 1
    fi
fi

# Get commit for tag
TAG_COMMIT=$(git rev-parse "${TAG}")
echo "Tag commit: ${TAG_COMMIT}"

# Update local manifest with new tag/commit
echo ""
echo "Updating local manifest..."
sed -i "s/tag: v[0-9.]*$/tag: ${TAG}/" "${SCRIPT_DIR}/${MANIFEST_FILE}"
sed -i "s/commit: [a-f0-9]*$/commit: ${TAG_COMMIT}/" "${SCRIPT_DIR}/${MANIFEST_FILE}"

# Check if local manifest changed
if ! git diff --quiet "${SCRIPT_DIR}/${MANIFEST_FILE}"; then
    echo "Local manifest updated. Committing..."
    git add "${SCRIPT_DIR}/${MANIFEST_FILE}"
    git commit -m "Update Flathub manifest for ${TAG}"
    git push
    echo "Pushed manifest update."
fi

# Regenerate cargo-sources.json
echo ""
echo "Regenerating cargo-sources.json..."
cd "${SCRIPT_DIR}"

# Use flatpak-cargo-generator if available, otherwise use flatpak-builder
if command -v flatpak-cargo-generator.py &> /dev/null; then
    flatpak-cargo-generator.py "${REPO_ROOT}/Cargo.lock" -o cargo-sources.json
elif [[ -f /usr/share/flatpak-builder-tools/cargo/flatpak-cargo-generator.py ]]; then
    python3 /usr/share/flatpak-builder-tools/cargo/flatpak-cargo-generator.py "${REPO_ROOT}/Cargo.lock" -o cargo-sources.json
else
    echo "Warning: flatpak-cargo-generator not found, using existing cargo-sources.json"
fi

cd "${REPO_ROOT}"

# Check if cargo-sources.json changed
if ! git diff --quiet "${SCRIPT_DIR}/cargo-sources.json" 2>/dev/null; then
    echo "cargo-sources.json updated. Committing..."
    git add "${SCRIPT_DIR}/cargo-sources.json"
    git commit -m "Update cargo-sources.json for ${TAG}"
    git push
    echo "Pushed cargo-sources.json update."
fi

echo ""
echo "Publishing ${TAG} (${TAG_COMMIT}) to Flathub..."

# Update flathub repo
cd "${FLATHUB_REPO}"
git fetch origin
git checkout master 2>/dev/null || git checkout main
git pull

# Check if branch already exists
BRANCH_NAME="${TAG}"
if git show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
    echo "Branch ${BRANCH_NAME} already exists locally, deleting..."
    git branch -D "${BRANCH_NAME}"
fi

# Create new branch
git checkout -b "${BRANCH_NAME}"

# Copy manifest and cargo-sources from main repo
cp "${REPO_ROOT}/flathub/${MANIFEST_FILE}" .
cp "${REPO_ROOT}/flathub/cargo-sources.json" .

# Lint the manifest
echo ""
echo "Linting manifest..."
if ! flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest "${MANIFEST_FILE}"; then
    echo "Error: Manifest failed linting"
    git checkout master
    exit 1
fi
echo "Manifest passed linting."

# Show diff
echo ""
echo "Changes to commit:"
git diff --stat
echo ""

# Check if there are changes
if git diff --quiet && git diff --cached --quiet; then
    echo "No changes to commit - Flathub already up to date."
    git checkout master
    exit 0
fi

# Commit
git add "${MANIFEST_FILE}" cargo-sources.json
git commit -m "Update to ${TAG}"

# Push branch
echo "Pushing branch ${BRANCH_NAME}..."
git push -u origin "${BRANCH_NAME}" --force

# Create PR
echo "Creating pull request..."
PR_URL=$(gh pr create --title "Update to ${TAG}" --body "Automated update to ${TAG}" --base master 2>&1) || true

if [[ "${PR_URL}" == *"already exists"* ]]; then
    echo "PR already exists, updating..."
    PR_URL=$(gh pr view --json url -q '.url' 2>/dev/null || echo "")
fi

# Return to master
git checkout master

echo ""
echo "Published ${TAG} to Flathub!"
if [[ -n "${PR_URL}" ]]; then
    echo "PR: ${PR_URL}"
fi
echo "Check build status at: https://buildbot.flathub.org/#/apps/io.github.danst0.passwordgenerator"
