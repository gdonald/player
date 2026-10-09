import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { alert, logIn, resetData, sql, storeSetting, storedSetting } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

async function expectDefaults(page: Page) {
  await expect(page.locator("#equalizer-panel")).toHaveCount(0);
  await expect(page.locator("#visualizer")).toBeVisible();
  await expect(page.locator(".playlist-list")).toBeVisible();
  await expect(page.locator("#theme-select")).toHaveValue("default");
  await expect(page.locator("#player-volume")).toHaveValue("1");
  await expect(page.locator("#mode-play")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".list-modes button", { hasText: "Songs" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#search")).toHaveValue("");
}

test("a new user starts with the defaults and no error", async ({ page }) => {
  await logIn(page);

  await expectDefaults(page);
  await expect(alert(page)).toHaveCount(0);
});

test("a new user's defaults are not written to the database", async ({ page }) => {
  await logIn(page);
  await expectDefaults(page);

  expect(await sql("SELECT name FROM user_settings")).toEqual([]);
});

test("malformed stored settings fall back to the defaults without an error", async ({ page }) => {
  await storeSetting("volume", "loud");
  await storeSetting("loop_mode", "sideways");
  await storeSetting("list_mode", "grid");
  await storeSetting("equalizer_open", "maybe");

  await logIn(page);

  await expectDefaults(page);
  await expect(alert(page)).toHaveCount(0);
});

test("settings this browser kept before are uploaded under their new names", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("player.mode", "loop-one");
    window.localStorage.setItem("player.volume", "0.4");
    window.localStorage.setItem("player.mp3s.mode", "albums");
    window.localStorage.setItem("player.visualizer.open", "false");
  });

  await logIn(page);

  await expect.poll(() => storedSetting("loop_mode")).toBe("loop-one");
  await expect.poll(() => storedSetting("volume")).toBe("0.4");
  await expect.poll(() => storedSetting("list_mode")).toBe("albums");
  await expect.poll(() => storedSetting("visualizer_open")).toBe("false");
});
