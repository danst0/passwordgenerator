#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "Cleaning build artifacts in ${REPO_ROOT}..."

paths=(
  "build-dir"
  "build-dir-test"
  "repo"
  "target"
  "passwordgenerator.flatpak"
)

for p in "${paths[@]}"; do
  if [[ -e "${p}" ]]; then
    rm -rf "${p}"
    echo "Removed: ${p}"
  fi
done

# Also clean any artifacts inside the flathub directory specifically
if [[ -d "${SCRIPT_DIR}/repo" ]]; then
  rm -rf "${SCRIPT_DIR}/repo"
  echo "Removed: ${SCRIPT_DIR}/repo"
fi
if [[ -f "${SCRIPT_DIR}/passwordgenerator.flatpak" ]]; then
  rm -f "${SCRIPT_DIR}/passwordgenerator.flatpak"
  echo "Removed: ${SCRIPT_DIR}/passwordgenerator.flatpak"
fi

echo "Cleanup complete."
