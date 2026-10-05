#!/usr/bin/env bash
# Starts the server for the browser tests on a fixed port, serving the built
# client, against the database in DATABASE_URL. Each test resets the data and
# the demo library itself (see tests/helpers.ts).
set -euo pipefail

cd "$(dirname "$0")/.."

: "${DATABASE_URL:?set DATABASE_URL to a Postgres database the browser tests may modify}"

# test.sh points CLIENT_DIST at the coverage build of the client.
CLIENT_DIST="${CLIENT_DIST:-$(pwd)/crates/client/dist}"

if [ ! -f "$CLIENT_DIST/index.html" ]; then
  (cd crates/client && trunk build --dist "$CLIENT_DIST")
fi

cargo build -p player-server --bin player

target/debug/player migrate
target/debug/player seed

export PORT=3100
export WEB_ROOT="$CLIENT_DIST"

exec target/debug/player serve
