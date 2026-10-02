#!/usr/bin/env bash
# The full gate: format the tree, then run everything test.sh runs.
set -euo pipefail

cd "$(dirname "$0")"

cargo fmt --all
./test.sh
