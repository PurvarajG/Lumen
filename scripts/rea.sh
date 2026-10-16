#!/bin/sh
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$project_dir/scripts/env.sh"
exec node "$project_dir/.tools/rea-runtime/node_modules/rea-agents/scripts/rea.mjs" "$@"
