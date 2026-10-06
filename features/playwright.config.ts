import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  globalSetup: "./global-setup.ts",
  // Each worker has its own server, database, and demo library (see
  // tests/worker.ts and tests/fixtures.ts). test.sh creates one database per
  // worker, so BROWSER_WORKERS must match there.
  fullyParallel: true,
  workers: Number(process.env.BROWSER_WORKERS ?? "10"),
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    trace: "retain-on-failure",
    launchOptions: { args: ["--autoplay-policy=no-user-gesture-required"] },
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
