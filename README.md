# Player

Web-based MP3 player. The server is Rust (axum, sqlx, Postgres). The browser client is Rust compiled to WebAssembly with Leptos, styled with Bootstrap.

![MP3 Player](https://raw.githubusercontent.com/gdonald/player/main/ss.png "MP3 Player")

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

Fill in `.env`, which git ignores. `PLAYER_SEED_USERNAME` and `PLAYER_SEED_PASSWORD` are the login the first run creates. The `player` binary reads `.env` at startup, and `run.sh` and `deploy.sh` source it. `.env.example` lists every setting.

```bash
./run.sh
```

The first run creates the `player_development` database (or the one named in `DATABASE_URL`), migrates it, and seeds it, with the seed login when one is set. Add another user with `player user add <username>`.

## Running

`./run.sh` starts the server on `PORT` (default 4000) and `trunk serve` on `HOST` (default 127.0.0.1) port 9000, which proxies `/api/` to the server. Open port 9000 on `HOST`, log in, add your music directory on the Sources page, and press Scan. Scan again after the music directory changes.

For a single process, build the client with `trunk build` in `crates/client` and run `player serve`, which serves `crates/client/dist` (or `WEB_ROOT`) on `PORT` (default 3000).

## Deploying

```bash
./deploy.sh
```

Deploys the committed `HEAD` to a Kubernetes cluster that Fleet manages from a devops repo. It builds the `Dockerfile` for `linux/amd64` and pushes it as `DEPLOY_IMAGE:<version>-<commit>`, with the version from `crates/server/Cargo.toml`. Then it changes the image in `DEPLOY_MANIFEST` in the `DEVOPS_REPO` checkout, commits and pushes that one file, waits for Fleet to apply it, and waits for the `DEPLOY_NAME` deployment in `DEPLOY_NAMESPACE` to roll out. It stops without deploying when either repo has uncommitted changes to what it uses. The five settings come from `.env`. It needs `docker` logged in to the registry and `kubectl` pointed at the cluster.

## Themes

Pick a theme from the menu at the left of the LIBRARY title bar. The choice is kept in the browser.

Each theme is one stylesheet, `crates/client/themes/theme-<name>.css`, holding every color, border, font, and control shape. `crates/client/assets/app.css` holds only layout, which every theme shares. To add a theme, copy `theme-default.css` to a new `theme-<name>.css`, change it, and add the name and label to `THEMES` in `crates/core/src/theme.rs`. A unit test fails when a file and the list disagree.

## Tests

```bash
./test.sh
```

Runs formatting and lint checks, the Rust tests with the coverage gate, and the Playwright browser tests in `features/`. Coverage must be 100% of lines, functions, and branches for the server, the shared crates, and the client. `./check.sh` formats the tree first.

The server and shared crates are measured with `cargo llvm-cov`, including the `player` binary, which `crates/server/tests/binary.rs` runs. The run prints coverage per file and the branch total, and writes an HTML report to `target/coverage/html/index.html` and an lcov file to `target/coverage/lcov.info`.

The browser tests run on `BROWSER_WORKERS` Playwright workers (10 by default, 2 in CI). Each worker has its own server (port 3100 plus the worker number), its own database (`player_e2e_0` and up, which `test.sh` creates), and its own demo library, and signs in through the API, except in the tests of the login form.

The client runs in the browser, so its coverage comes from the Playwright tests. `test.sh` builds a copy of the client with coverage counters (nightly, the `coverage` feature, and `scripts/coverage-rustc.sh`), runs the browser tests against it, and saves the counters before every page load and at the end of each test. The counters are merged and reported against the client's coverage map, and the report is written to `target/coverage/client/index.html`.

`.github/workflows/ci.yml` runs `./test.sh` on every push to `main` and every pull request, against a Postgres service, and uploads the coverage report and the Playwright report as artifacts.

## iOS

The [iOS app](https://github.com/gdonald/player-ios) works with the same API.

## License

[![GitHub](https://img.shields.io/github/license/gdonald/player?color=0000bb)](https://github.com/gdonald/player/blob/main/LICENSE)
