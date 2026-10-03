import { Page, expect, test } from "@playwright/test";
import { logIn, resetData } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

function pageOverflow(page: Page) {
  return page.evaluate(() => document.documentElement.scrollHeight - document.documentElement.clientHeight);
}

test("the page fits the window with the equalizer closed", async ({ page }) => {
  await expect.poll(() => pageOverflow(page)).toBe(0);
});

test("the page fits the window with the equalizer open", async ({ page }) => {
  await page.click("#equalizer-toggle");
  await expect(page.locator("#equalizer-panel")).toBeVisible();

  await expect.poll(() => pageOverflow(page)).toBe(0);
});

test("the page fits a short window", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 500 });
  await page.click("#equalizer-toggle");

  await expect.poll(() => pageOverflow(page)).toBe(0);
});

test("the library buttons sit in one row and mark the open section", async ({ page }, testInfo) => {
  const buttons = page.locator(".library-button");
  await expect(buttons).toHaveText([/MP3s/, /Playlists/, /Sources/]);

  const tops = await buttons.evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect().top));
  expect(new Set(tops).size).toBe(1);

  await expect(buttons.nth(0)).toHaveAttribute("aria-pressed", "true");

  const truncated = await page.locator(".library-label").evaluateAll((labels) =>
    labels.filter((label) => label.scrollWidth > label.clientWidth).length,
  );
  expect(truncated).toBe(0);

  await buttons.nth(2).click();

  await expect(buttons.nth(2)).toHaveAttribute("aria-pressed", "true");
  await expect(buttons.nth(0)).toHaveAttribute("aria-pressed", "false");

  await page.screenshot({ path: testInfo.outputPath("library.png") });
});

test("no title names the old product", async ({ page }) => {
  await expect(page.locator("body")).not.toContainText(/winamp/i);
  await expect(page.locator(".winamp-titlebar span")).toHaveText(["LIBRARY", "PLAYLIST", "VISUALIZATION", "EQUALIZER", "PLAYER"]);
});

test("the selected library button's icon is green", async ({ page }) => {
  const color = await page.locator(".library-button.winamp-lit i").evaluate((icon) => getComputedStyle(icon).color);

  expect(color).toBe("rgb(44, 255, 44)");
});

test("the PL button hides and shows the playlist, and the choice is kept", async ({ page }) => {
  await expect(page.locator(".playlist-list")).toBeVisible();

  await page.click("#playlist-toggle");
  await expect(page.locator(".playlist-list")).toHaveCount(0);
  await expect(page.locator("#playlist-toggle")).toHaveAttribute("aria-expanded", "false");

  await page.reload();
  await expect(page.locator(".library-nav")).toBeVisible();
  await expect(page.locator(".playlist-list")).toHaveCount(0);

  await page.click("#playlist-toggle");
  await expect(page.locator(".playlist-list")).toBeVisible();
});
