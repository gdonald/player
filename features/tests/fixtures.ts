import fs from "node:fs";
import path from "node:path";
import { Page, TestInfo, test as base, expect } from "@playwright/test";

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

export const test = base.extend({
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
