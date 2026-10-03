# Player

Web-based MP3 player. The server is Rust (axum, sqlx, Postgres). The browser client is Rust compiled to WebAssembly with Leptos, styled with Bootstrap.

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss.png "MP3 Player")

## Requirements

- Rust with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) to build the client
- Postgres
- Node, for the browser tests only

## Setup

```bash
./run.sh
```

The first run creates the `player_development` database (or the one named in `DATABASE_URL`), migrates it, and seeds it with the user `gd`, password `changeme`. Add another user with `player user add <username>`.

## Running

`./run.sh` starts the server on `PORT` (default 4000) and `trunk serve` on `HOST` (default 10.0.0.45) port 9000, which proxies `/api/` to the server. Open http://10.0.0.45:9000, log in, add your music directory on the Sources page, and press Scan. Scan again after the music directory changes.

For a single process, build the client with `trunk build` in `crates/client` and run `player serve`, which serves `crates/client/dist` (or `WEB_ROOT`) on `PORT` (default 3000).

## Tests

```bash
./test.sh
```

Runs formatting and lint checks, the Rust tests with the coverage gate (100% of lines, functions, and branches), and the Playwright browser tests in `features/`. `./check.sh` formats the tree first.

## iOS

The [iOS app](https://github.com/gdonald/player-ios) works with the same API.

## License

[![GitHub](https://img.shields.io/github/license/gdonald/player?color=0000bb)](https://github.com/gdonald/player/blob/main/LICENSE)
