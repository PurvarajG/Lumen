#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
export PATH
cd "$PROJECT_ROOT"
exec cargo run -p lumen-app -- "$@"
