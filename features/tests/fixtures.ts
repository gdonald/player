import { ChildProcess, spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs";
import path from "node:path";
import { Page, TestInfo, test as base, expect } from "@playwright/test";
import { PASSWORD, USERNAME } from "./helpers";
import { BASE_URL, DATABASE_URL, PORT } from "./worker";

/// Set by test.sh when the tests run against the coverage build of the client.
const COVERAGE_DIR = process.env.CLIENT_COVERAGE_DIR;

/// Saves the client's coverage counters as a .profraw file. A page load starts
/// the counters over, so this runs before every reload and navigation and at
/// the end of each test.
async function saveCoverage(page: Page, testInfo: TestInfo, saved: { count: number }) {
  if (!COVERAGE_DIR || page.isClosed()) {
    return;
  }

  const bytes = await page
    .evaluate(() => {
      const bindings = (window as unknown as { wasmBindings?: { player_coverage?: () => Uint8Array } }).wasmBindings;
      return bindings?.player_coverage ? Array.from(bindings.player_coverage()) : null;
    })
    .catch(() => null);

  if (!bytes) {
    return;
  }

  saved.count += 1;
  fs.mkdirSync(COVERAGE_DIR, { recursive: true });
  const name = `${testInfo.testId}-${saved.count}.profraw`;
  fs.writeFileSync(path.join(COVERAGE_DIR, name), Buffer.from(bytes));
}

const SERVER_START_MILLISECONDS = 240_000;

/// Polls until the server answers, since it migrates and seeds before it
/// listens, and fails at once if the server exits instead.
async function waitForServer(server: ChildProcess) {
  const deadline = Date.now() + SERVER_START_MILLISECONDS;

  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`the server for port ${PORT} exited with status ${server.exitCode}`);
    }

    const ready = await fetch(`${BASE_URL}/robots.txt`).then((response) => response.ok, () => false);
    if (ready) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }

  throw new Error(`the server on port ${PORT} did not start`);
}

export const test = base.extend<object, { server: void }>({
  server: [
    async ({}, use) => {
      const server = spawn("bash", [path.resolve(__dirname, "../run-app.sh")], {
        env: {
          ...process.env,
          DATABASE_URL,
          PORT: String(PORT),
          RUST_LOG: "warn",
          PLAYER_SEED_USERNAME: USERNAME,
          PLAYER_SEED_PASSWORD: PASSWORD,
        },
        stdio: ["ignore", "ignore", "inherit"],
      });

      await waitForServer(server);
      await use();

      server.kill("SIGINT");
      await once(server, "exit");
    },
    { scope: "worker", auto: true, timeout: SERVER_START_MILLISECONDS },
  ],

  baseURL: async ({}, use) => {
    await use(BASE_URL);
  },

  page: async ({ page }, use, testInfo) => {
    const saved = { count: 0 };
    const reload = page.reload.bind(page);
    const goto = page.goto.bind(page);

    page.reload = async (options) => {
      await saveCoverage(page, testInfo, saved);
      return reload(options);
    };
    page.goto = async (url, options) => {
      await saveCoverage(page, testInfo, saved);
      return goto(url, options);
    };

    await use(page);

    await saveCoverage(page, testInfo, saved);
  },
});

export { expect };
