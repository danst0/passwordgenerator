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

# Check for gh CLI
if ! command -v gh &> /dev/null; then
    echo "Error: gh (GitHub CLI) is required"
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

# Check if branch already exists
BRANCH_NAME="${LATEST_TAG}"
if git show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
    echo "Branch ${BRANCH_NAME} already exists locally, deleting..."
    git branch -D "${BRANCH_NAME}"
fi

# Create new branch
git checkout -b "${BRANCH_NAME}"

# Copy manifest from main repo
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
    echo "No changes to commit."
    git checkout master
    exit 0
fi

# Commit
git add io.github.danst0.passwordgenerator.yml cargo-sources.json
git commit -m "Update to ${LATEST_TAG}"

# Push branch
echo "Pushing branch ${BRANCH_NAME}..."
git push -u origin "${BRANCH_NAME}" --force

# Create PR
echo "Creating pull request..."
PR_URL=$(gh pr create --title "Update to ${LATEST_TAG}" --body "Automated update to ${LATEST_TAG}" --base master 2>&1) || true

if [[ "${PR_URL}" == *"already exists"* ]]; then
    echo "PR already exists, updating..."
    PR_URL=$(gh pr view --json url -q '.url' 2>/dev/null || echo "")
fi

# Return to master
git checkout master

echo ""
echo "Published ${LATEST_TAG} to Flathub!"
if [[ -n "${PR_URL}" ]]; then
    echo "PR: ${PR_URL}"
fi
echo "Check build status at: https://buildbot.flathub.org/#/apps/io.github.danst0.passwordgenerator"
