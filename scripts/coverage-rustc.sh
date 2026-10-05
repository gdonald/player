#!/usr/bin/env bash
# RUSTC_WRAPPER for the client coverage build. Only the client crate is
# instrumented: dependencies such as wasm-streams link their own WebAssembly
# modules, which fail to link with coverage counters and no profiling runtime.
# The client's LLVM IR is kept so the report step can read its coverage map.
set -euo pipefail

rustc="$1"
shift

previous=""
for argument in "$@"; do
  if [ "$previous" = "--crate-name" ] && [ "$argument" = "player_client" ]; then
    exec "$rustc" "$@" \
      -Cinstrument-coverage -Zcoverage-options=branch -Zno-profiler-runtime \
      -Clink-args=--no-gc-sections --emit=llvm-ir
  fi
  previous="$argument"
done

exec "$rustc" "$@"
