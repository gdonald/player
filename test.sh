#!/usr/bin/env bash
# The whole suite: format and lints, Rust tests with the coverage gate, then the
# browser tests against a built client and server.
#
#   DATABASE_URL       Postgres database the Rust tests start from (#[sqlx::test]
#                      makes a throwaway database per test off it).
#   E2E_DATABASE_URL   Postgres database the browser tests run against.
set -euo pipefail

cd "$(dirname "$0")"

DB_USER="$(whoami)"
export DATABASE_URL="${DATABASE_URL:-postgres://$DB_USER@localhost/player_test}"
E2E_DATABASE_URL="${E2E_DATABASE_URL:-postgres://$DB_USER@localhost/player_e2e}"
COVERAGE_DIR="target/coverage"
COVERAGE_IGNORE='crates/client/|crates/server/src/main\.rs|crates/server/examples/|crates/server/tests/'

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

if ! rustup component list --toolchain nightly --installed 2>/dev/null | grep -q '^llvm-tools'; then
  echo "branch coverage needs nightly with llvm-tools: rustup component add llvm-tools-preview --toolchain nightly" >&2
  exit 1
fi

if [ ! -d features/node_modules/@playwright/test ]; then
  echo "browser tests need their packages: (cd features && npm install && npx playwright install chromium)" >&2
  exit 1
fi

step "resetting test databases"
dropdb --if-exists --force "${DATABASE_URL##*/}"
createdb "${DATABASE_URL##*/}"
dropdb --if-exists --force "${E2E_DATABASE_URL##*/}"
createdb "${E2E_DATABASE_URL##*/}"

step "format and lints"
cargo fmt --all --check
cargo clippy --workspace --exclude player-client --all-targets -- -D warnings
cargo clippy -p player-client --target wasm32-unknown-unknown -- -D warnings

step "rust tests with line and function coverage"
cargo llvm-cov --workspace --exclude player-client \
  --ignore-filename-regex "$COVERAGE_IGNORE" \
  --html --output-dir "$COVERAGE_DIR" \
  --fail-uncovered-lines 0 --fail-uncovered-functions 0

# --branch is unstable, so branches come from a nightly run in its own target
# directory. Nightly's per-line counts land on blank lines, so lines are gated
# on the stable run above.
step "rust tests with branch coverage (nightly)"
CARGO_TARGET_DIR=target/nightly cargo +nightly llvm-cov --workspace --exclude player-client --branch \
  --ignore-filename-regex "$COVERAGE_IGNORE" \
  --json --summary-only --output-path "$COVERAGE_DIR/branches.json"

jq -e '.data[0].totals.branches | if .covered == .count then true
       else ("\(.count - .covered) of \(.count) branches uncovered\n" | halt_error) end' \
  "$COVERAGE_DIR/branches.json" >/dev/null

step "building the client"
(cd crates/client && trunk build)

step "browser tests"
(cd features && DATABASE_URL="$E2E_DATABASE_URL" npx playwright test)

rm -f default*.profraw

printf '\n\033[1;32mAll tests passed.\033[0m\n'
printf '  coverage report:  file://%s/%s/html/index.html\n' "$PWD" "$COVERAGE_DIR"
printf '  browser report:   file://%s/features/playwright-report/index.html\n' "$PWD"
