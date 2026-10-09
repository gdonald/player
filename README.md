# Player

Web-based MP3 player. The server is Rust (axum, sqlx, Postgres). The browser client is Rust compiled to WebAssembly with Leptos, styled with Bootstrap.

## Requirements

- Rust with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) to build the client
- Postgres
- Node, for the browser tests only
- For the coverage gates in `./test.sh`: nightly Rust with `llvm-tools-preview` and the `wasm32-unknown-unknown` target, `cargo-llvm-cov`, `jq`, and a clang that targets wasm32

## Setup

```bash
cp .env.example .env
```

Fill in `.env`. `PLAYER_SEED_USERNAME` and `PLAYER_SEED_PASSWORD` set the login the first run creates.

```bash
./run.sh
```

The first run creates the `player_development` database (or the one named in `DATABASE_URL`), migrates it, and seeds it. Add another user with `player user add <username>`.

## Running

`./run.sh` starts the server on `PORT` (default 4000) and `trunk serve` on `HOST` (default 127.0.0.1) port 9000, which proxies `/api/` to the server. Open port 9000 on `HOST`, log in, add your music directory on the Sources page, and press Scan. Scan again after the music directory changes. Remove takes a source's songs out of the library, playlists, and queue, and leaves the files on disk.

For a single process, build the client with `trunk build` in `crates/client` and run `player serve`, which serves `crates/client/dist` (or `WEB_ROOT`) on `PORT` (default 3000).

## Themes

Pick a theme from the menu at the right of the player's title bar, next to the logout button. The choice is kept in the browser.

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss1.png "MP3 Player")

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss2.png "MP3 Player")

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss3.png "MP3 Player")

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss4.png "MP3 Player")

Each theme is one stylesheet in `crates/client/themes/`. To add one, copy `theme-default.css` to `theme-<name>.css`, edit it, and add its name and label to `THEMES` in `crates/core/src/theme.rs`.

## Tests

```bash
./test.sh
```

Runs formatting and lint checks, the Rust tests, and the Playwright browser tests in `features/`, and fails below 100% coverage of lines, functions, and branches. `./check.sh` formats the tree first.

Coverage reports go to `target/coverage/html/index.html` for the server and `target/coverage/client/index.html` for the client. The browser tests run on 10 workers, or as many as `BROWSER_WORKERS` sets.

CI runs `./test.sh` on every push to `main` and every pull request.

## Deploying

```bash
./deploy.sh
```

Builds the committed `HEAD` for `linux/amd64`, pushes it as `DEPLOY_IMAGE:<version>-<commit>`, points `DEPLOY_MANIFEST` in the `DEVOPS_REPO` checkout at it, and waits for the `DEPLOY_NAME` deployment in `DEPLOY_NAMESPACE` to roll out. It stops if either repo has uncommitted changes. It needs `docker` logged in to the registry and `kubectl` pointed at the cluster.

## iOS

The [iOS app](https://github.com/gdonald/player-ios) works with the same API.

## License

[![GitHub](https://img.shields.io/github/license/gdonald/player?color=0000bb)](https://github.com/gdonald/player/blob/main/LICENSE)
