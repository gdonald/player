#!/usr/bin/env bash
# Starts the server for the browser tests on a fixed port, serving the built
# client, against the database in DATABASE_URL. Each test resets the data and
# the demo library itself (see tests/helpers.ts).
set -euo pipefail

cd "$(dirname "$0")/.."

: "${DATABASE_URL:?set DATABASE_URL to a Postgres database the browser tests may modify}"

if [ ! -f crates/client/dist/index.html ]; then
  (cd crates/client && trunk build)
fi

cargo build -p player-server --bin player --example demo_library

target/debug/player migrate
target/debug/player seed

export PORT=3100
export WEB_ROOT="$(pwd)/crates/client/dist"

exec target/debug/player serve
