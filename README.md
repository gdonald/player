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
./run.sh
```

The first run creates the `player_development` database (or the one named in `DATABASE_URL`), migrates it, and seeds it with the user `gd`, password `changeme`. Add another user with `player user add <username>`.

## Running

`./run.sh` starts the server on `PORT` (default 4000) and `trunk serve` on `HOST` (default 10.0.0.45) port 9000, which proxies `/api/` to the server. Open http://10.0.0.45:9000, log in, add your music directory on the Sources page, and press Scan. Scan again after the music directory changes.

For a single process, build the client with `trunk build` in `crates/client` and run `player serve`, which serves `crates/client/dist` (or `WEB_ROOT`) on `PORT` (default 3000).

## Deploying

```bash
./deploy.sh
```

Deploys the committed `HEAD` to the home lab cluster. It builds the `Dockerfile` for `linux/amd64` and pushes it to Harbor as `harbor.home.gregdonald.com/player/player:<version>-<commit>`, with the version from `crates/server/Cargo.toml`. Then it changes the image in the devops repo's `fleet/player/deployment.yaml`, commits and pushes that one file, waits for Fleet to apply it, and waits for the new pod. It stops without deploying when either repo has uncommitted changes to what it uses. `DEVOPS_REPO` names the devops checkout (default `~/workspace/devops`). It needs `docker` logged in to Harbor and `kubectl` pointed at the cluster.

## Themes

Pick a theme from the menu at the left of the LIBRARY title bar. The choice is kept in the browser.

Each theme is one stylesheet, `crates/client/themes/theme-<name>.css`, holding every color, border, font, and control shape. `crates/client/assets/app.css` holds only layout, which every theme shares. To add a theme, copy `theme-default.css` to a new `theme-<name>.css`, change it, and add the name and label to `THEMES` in `crates/core/src/theme.rs`. A unit test fails when a file and the list disagree.

## Tests

```bash
./test.sh
```

Runs formatting and lint checks, the Rust tests with the coverage gate, and the Playwright browser tests in `features/`. Coverage must be 100% of lines, functions, and branches for the server, the shared crates, and the client. `./check.sh` formats the tree first.

The server and shared crates are measured with `cargo llvm-cov`, including the `player` binary, which `crates/server/tests/binary.rs` runs. The run prints coverage per file and the branch total, and writes an HTML report to `target/coverage/html/index.html` and an lcov file to `target/coverage/lcov.info`.

The client runs in the browser, so its coverage comes from the Playwright tests. `test.sh` builds a copy of the client with coverage counters (nightly, the `coverage` feature, and `scripts/coverage-rustc.sh`), runs the browser tests against it, and saves the counters before every page load and at the end of each test. The counters are merged and reported against the client's coverage map, and the report is written to `target/coverage/client/index.html`.

`.github/workflows/ci.yml` runs `./test.sh` on every push to `main` and every pull request, against a Postgres service, and uploads the coverage report and the Playwright report as artifacts.

## iOS

The [iOS app](https://github.com/gdonald/player-ios) works with the same API.

## License

[![GitHub](https://img.shields.io/github/license/gdonald/player?color=0000bb)](https://github.com/gdonald/player/blob/main/LICENSE)
