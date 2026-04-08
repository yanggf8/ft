#!/bin/bash
# Install git hooks from .githooks/ into .git/hooks/
# Run once after cloning: bash .githooks/install.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOKS_DIR="$(git rev-parse --git-dir)/hooks"

for hook in "$SCRIPT_DIR"/*; do
  name=$(basename "$hook")
  [[ "$name" == "install.sh" ]] && continue
  cp "$hook" "$HOOKS_DIR/$name"
  chmod +x "$HOOKS_DIR/$name"
  echo "✅ Installed $name"
done
