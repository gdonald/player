#!/usr/bin/env bash
# Local development: the server on PORT (default 4000) and `trunk serve` on
# HOST (default 10.0.0.45) port 9000, which rebuilds the client on change and
# proxies /api/ to the server.
set -euo pipefail

cd "$(dirname "$0")"

export DATABASE_URL="${DATABASE_URL:-postgres://$(whoami)@localhost/player_development}"
DATABASE_NAME="${DATABASE_URL##*/}"
DATABASE_NAME="${DATABASE_NAME%%\?*}"

if ! psql -lqt | cut -d '|' -f 1 | grep -qw "$DATABASE_NAME"; then
  createdb "$DATABASE_NAME"
  cargo run -p player-server --bin player -- migrate
  cargo run -p player-server --bin player -- seed
fi

export PORT="${PORT:-4000}"
HOST="${HOST:-10.0.0.45}"

for port in "$PORT" 9000; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null; then
    echo "port $port is in use by $(lsof -nP -iTCP:"$port" -sTCP:LISTEN | awk 'NR == 2 {print $1 " (pid " $2 ")"}')" >&2
    [ "$port" = "$PORT" ] && echo "run on another port with PORT=<port> ./run.sh" >&2
    exit 1
  fi
done

trap 'kill 0' EXIT

(cd crates/client && trunk serve --address "$HOST" --proxy-rewrite /api/ --proxy-backend "http://127.0.0.1:$PORT/api/") &

cargo run -p player-server --bin player -- serve
