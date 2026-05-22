#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Install project-local AccelFury `af-*` Codex skills into CODEX_HOME.

set -euo pipefail

repo="$(git rev-parse --show-toplevel)"
cd "$repo"

codex_home="${CODEX_HOME:-$HOME/.codex}"
dest="$codex_home/skills"

shopt -s nullglob
skill_dirs=(skills/af-*)
if [[ ${#skill_dirs[@]} -eq 0 ]]; then
  echo "FATAL: no project Codex skills found under skills/af-*" >&2
  exit 3
fi

mkdir -p "$dest"

for dir in "${skill_dirs[@]}"; do
  name="$(basename "$dir")"
  mkdir -p "$dest/$name"
  rsync -a --delete "$dir/" "$dest/$name/"
  echo "installed $name -> $dest/$name"
done

echo "installed ${#skill_dirs[@]} af Codex skill(s)"
