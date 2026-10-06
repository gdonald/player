#!/usr/bin/env bash
# The whole suite: format and lints, Rust tests with the coverage gate, then the
# browser tests against a built client and server.
#
#   DATABASE_URL       Postgres database the Rust tests start from (#[sqlx::test]
#                      makes a throwaway database per test off it).
#   E2E_DATABASE_URL   Base name of the browser test databases, one per worker
#                      (<name>_0 up to <name>_9).
#   BROWSER_WORKERS    Browser test workers and databases (default 10).
set -euo pipefail

cd "$(dirname "$0")"

DB_USER="$(whoami)"
export DATABASE_URL="${DATABASE_URL:-postgres://$DB_USER@localhost/player_test}"
E2E_DATABASE_URL="${E2E_DATABASE_URL:-postgres://$DB_USER@localhost/player_e2e}"
export BROWSER_WORKERS="${BROWSER_WORKERS:-10}"
COVERAGE_DIR="target/coverage"
COVERAGE_IGNORE='crates/client/|crates/server/examples/|crates/server/tests/'
CLIENT_COVERAGE="$PWD/target/client-coverage"
NIGHTLY_LLVM="$(rustc +nightly --print sysroot)/lib/rustlib/$(rustc +nightly -vV | awk '/^host:/ {print $2}')/bin"

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

if ! rustup component list --toolchain nightly --installed 2>/dev/null | grep -q '^llvm-tools'; then
  echo "branch coverage needs nightly with llvm-tools: rustup component add llvm-tools-preview --toolchain nightly" >&2
  exit 1
fi

if ! rustup target list --toolchain nightly --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
  echo "client coverage needs the wasm32 target on nightly: rustup target add wasm32-unknown-unknown --toolchain nightly" >&2
  exit 1
fi

if [ ! -d features/node_modules/@playwright/test ]; then
  echo "browser tests need their packages: (cd features && npm install && npx playwright install chromium)" >&2
  exit 1
fi

step "resetting test databases"
dropdb --if-exists --force "${DATABASE_URL##*/}"
createdb "${DATABASE_URL##*/}"
# One database per browser test worker: <E2E database>_0 up to _<BROWSER_WORKERS - 1>.
for worker in $(seq 0 $((BROWSER_WORKERS - 1))); do
  dropdb --if-exists --force "${E2E_DATABASE_URL##*/}_$worker"
  createdb "${E2E_DATABASE_URL##*/}_$worker"
done

step "format and lints"
cargo fmt --all --check
cargo clippy --workspace --exclude player-client --all-targets -- -D warnings
cargo clippy -p player-client --target wasm32-unknown-unknown -- -D warnings

step "rust tests with line and function coverage"
cargo llvm-cov --workspace --exclude player-client \
  --ignore-filename-regex "$COVERAGE_IGNORE" \
  --html --output-dir "$COVERAGE_DIR" \
  --fail-uncovered-lines 0 --fail-uncovered-functions 0

step "coverage by file"
cargo llvm-cov report --ignore-filename-regex "$COVERAGE_IGNORE" --summary-only
cargo llvm-cov report --ignore-filename-regex "$COVERAGE_IGNORE" --lcov --output-path "$COVERAGE_DIR/lcov.info"

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

jq -r '.data[0].totals.branches | "branches: \(.covered) of \(.count) covered"' "$COVERAGE_DIR/branches.json"

step "building the client"
(cd crates/client && trunk build)

# The client runs in the browser, so its coverage comes from the browser tests
# running against a build with coverage counters. Only the client crate is
# instrumented (see scripts/coverage-rustc.sh), and minicov's runtime is
# archived with llvm-ar, since the system ar drops WebAssembly objects.
step "building the client with coverage counters (nightly)"
rm -rf "$CLIENT_COVERAGE/dist" "$CLIENT_COVERAGE/profraw"
(cd crates/client && RUSTUP_TOOLCHAIN=nightly \
  RUSTC_WRAPPER="$PWD/../../scripts/coverage-rustc.sh" \
  AR_wasm32_unknown_unknown="$NIGHTLY_LLVM/llvm-ar" \
  CARGO_TARGET_DIR="$CLIENT_COVERAGE" \
  trunk build --features coverage --dist "$CLIENT_COVERAGE/dist")

step "browser tests"
(cd features && DATABASE_URL="$E2E_DATABASE_URL" \
  CLIENT_DIST="$CLIENT_COVERAGE/dist" CLIENT_COVERAGE_DIR="$CLIENT_COVERAGE/profraw" \
  npx playwright test)

# llvm-cov cannot read a WebAssembly object, so the client's LLVM IR is
# compiled to an ELF object, which carries the same coverage map. Mach-O
# cannot hold the map's COMDAT groups, so ELF is used on every host.
step "client coverage"
CLIENT_IR="$(ls -t $(find "$CLIENT_COVERAGE" -name player_client.ll) | head -1)"
"$NIGHTLY_LLVM/llc" -filetype=obj -mtriple=x86_64-unknown-linux-gnu "$CLIENT_IR" \
  -o "$CLIENT_COVERAGE/player_client.o"
"$NIGHTLY_LLVM/llvm-profdata" merge -sparse "$CLIENT_COVERAGE"/profraw/*.profraw \
  -o "$CLIENT_COVERAGE/client.profdata"

CLIENT_REPORT=("$CLIENT_COVERAGE/player_client.o" -instr-profile="$CLIENT_COVERAGE/client.profdata")
"$NIGHTLY_LLVM/llvm-cov" report "${CLIENT_REPORT[@]}"
"$NIGHTLY_LLVM/llvm-cov" show "${CLIENT_REPORT[@]}" -format=html -output-dir="$COVERAGE_DIR/client"
"$NIGHTLY_LLVM/llvm-cov" export "${CLIENT_REPORT[@]}" -summary-only > "$COVERAGE_DIR/client.json"

jq -e '.data[0].totals | [.lines, .functions, .branches] | map(select(.covered != .count))
       | if length == 0 then true
         else ("client coverage is below 100%\n" | halt_error) end' \
  "$COVERAGE_DIR/client.json" >/dev/null

rm -f default*.profraw

printf '\n\033[1;32mAll tests passed.\033[0m\n'
printf '  coverage report:  file://%s/%s/html/index.html\n' "$PWD" "$COVERAGE_DIR"
printf '  coverage lcov:    %s/%s/lcov.info\n' "$PWD" "$COVERAGE_DIR"
printf '  client coverage:  file://%s/%s/client/index.html\n' "$PWD" "$COVERAGE_DIR"
printf '  browser report:   file://%s/features/playwright-report/index.html\n' "$PWD"
