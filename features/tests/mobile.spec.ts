import { Page, expect, test } from "@playwright/test";
import { logIn, menu, resetData, sql } from "./helpers";

test.use({ viewport: { width: 390, height: 844 } });

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

function horizontalOverflow(page: Page) {
  return page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
}

function top(page: Page, selector: string) {
  return page.locator(selector).evaluate((element) => element.getBoundingClientRect().top);
}

test("the MP3s page has no horizontal scroll", async ({ page }) => {
  await expect(page.locator("#mp3s")).toBeVisible();

  expect(await horizontalOverflow(page)).toBe(0);
});

test("a long source path does not cause horizontal scroll", async ({ page }) => {
  await sql("INSERT INTO sources (path) VALUES ($1)", [
    "/a/very/long/path/without/any/spaces/to/wrap/at/music/library/collection/archive",
  ]);
  await menu(page, "Sources").click();
  await expect(page.locator("#sources")).toBeVisible();

  expect(await horizontalOverflow(page)).toBe(0);
});

test("sections stack as player, library, content, then the side windows", async ({ page }) => {
  const order = [
    await top(page, ".area-player"),
    await top(page, ".area-library"),
    await top(page, ".area-content"),
    await top(page, ".area-windows"),
  ];

  expect([...order].sort((first, second) => first - second)).toEqual(order);
});

test("the player stays at the top while the page scrolls", async ({ page }) => {
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));

  await expect.poll(() => top(page, ".area-player")).toBe(0);
});

test("the album and track columns are hidden", async ({ page }) => {
  await expect(page.locator("#mp3s")).toBeVisible();

  await expect(page.locator("#mp3s th.col-album")).toBeHidden();
  await expect(page.locator("#mp3s th.col-track")).toBeHidden();
});

test("the content fills the screen when the side windows are closed", async ({ page }) => {
  await page.click("#playlist-toggle");
  await page.click("#visualizer-toggle");
  await menu(page, "Sources").click();
  await expect(page.locator("#sources")).toBeVisible();

  const bottom = await page.locator(".area-windows").evaluate((element) => element.getBoundingClientRect().bottom);

  expect(bottom).toBeGreaterThanOrEqual(844 - 1);
});

test("the content area has the window border", async ({ page }) => {
  const width = await page.locator(".area-content").evaluate((element) => getComputedStyle(element).borderTopWidth);

  expect(width).toBe("1px");
});
