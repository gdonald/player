#!/usr/bin/env bash
# Starts one browser test worker's server on PORT, serving the built client,
# against the database in DATABASE_URL. global-setup.ts builds the server and
# the client first, and each test resets the data and the demo library itself
# (see tests/helpers.ts).
set -euo pipefail

cd "$(dirname "$0")/.."

: "${DATABASE_URL:?set DATABASE_URL to a Postgres database the browser tests may modify}"
: "${PORT:?set PORT to the port this server listens on}"

# test.sh points CLIENT_DIST at the coverage build of the client.
export WEB_ROOT="${CLIENT_DIST:-$(pwd)/crates/client/dist}"

target/debug/player migrate
target/debug/player seed

exec target/debug/player serve
