import { Page, expect, test } from "@playwright/test";
import { logIn, resetData } from "./helpers";

const DEFAULT_TITLE_COLOR = "rgb(217, 199, 108)";
const CHARCOAL_TITLE_COLOR = "rgb(211, 212, 215)";
const CHARCOAL_DISPLAY_COLOR = "rgb(162, 193, 204)";
const STEEL_DISPLAY_COLOR = "rgb(244, 244, 244)";
const STEEL_LIST_COLOR = "rgb(185, 241, 185)";
const SAPPHIRE_DISPLAY_COLOR = "rgb(255, 255, 255)";
const SAPPHIRE_LIST_BACKGROUND = "rgb(28, 61, 125)";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

function themeLink(page: Page) {
  return page.locator("link#theme");
}

function titleColor(page: Page) {
  return page.locator(".library .winamp-titlebar span").evaluate((title) => getComputedStyle(title).color);
}

function pageOverflow(page: Page) {
  return page.evaluate(() => document.documentElement.scrollHeight - document.documentElement.clientHeight);
}

test("the default theme is in place at first", async ({ page }) => {
  await expect(themeLink(page)).toHaveAttribute("href", "/themes/theme-default.css");
  await expect(page.locator("#theme-select")).toHaveValue("default");

  await expect.poll(() => titleColor(page)).toBe(DEFAULT_TITLE_COLOR);
});

test("choosing a theme swaps its stylesheet in", async ({ page }, testInfo) => {
  await page.selectOption("#theme-select", "charcoal");

  await expect(themeLink(page)).toHaveAttribute("href", "/themes/theme-charcoal.css");
  await expect.poll(() => titleColor(page)).toBe(CHARCOAL_TITLE_COLOR);

  await page.screenshot({ path: testInfo.outputPath("charcoal.png") });
});

test("the charcoal theme shows display text in pale blue-gray", async ({ page }) => {
  await page.selectOption("#theme-select", "charcoal");

  for (const selector of ["#player-position", "#player-title", "#player-duration", ".library-count"]) {
    await expect
      .poll(() => page.locator(selector).first().evaluate((element) => getComputedStyle(element).color))
      .toBe(CHARCOAL_DISPLAY_COLOR);
  }
});

test("the steel theme shows white display text and green playlist text", async ({ page }, testInfo) => {
  await page.selectOption("#theme-select", "steel");
  await expect(themeLink(page)).toHaveAttribute("href", "/themes/theme-steel.css");

  for (const selector of ["#player-position", "#player-title", "#player-duration", ".library-count"]) {
    await expect
      .poll(() => page.locator(selector).first().evaluate((element) => getComputedStyle(element).color))
      .toBe(STEEL_DISPLAY_COLOR);
  }
  await expect
    .poll(() => page.locator("#mp3s tbody td").nth(1).evaluate((element) => getComputedStyle(element).color))
    .toBe(STEEL_LIST_COLOR);

  await page.screenshot({ path: testInfo.outputPath("steel.png") });
});

test("the sapphire theme shows white display text and a blue playlist", async ({ page }, testInfo) => {
  await page.selectOption("#theme-select", "sapphire");
  await expect(themeLink(page)).toHaveAttribute("href", "/themes/theme-sapphire.css");

  for (const selector of ["#player-position", "#player-title", "#player-duration", ".library-count"]) {
    await expect
      .poll(() => page.locator(selector).first().evaluate((element) => getComputedStyle(element).color))
      .toBe(SAPPHIRE_DISPLAY_COLOR);
  }
  await expect
    .poll(() => page.locator(".playlist-list").evaluate((element) => getComputedStyle(element).backgroundColor))
    .toBe(SAPPHIRE_LIST_BACKGROUND);

  await page.screenshot({ path: testInfo.outputPath("sapphire.png") });
});

test("the chosen theme is kept across a reload", async ({ page }) => {
  await page.selectOption("#theme-select", "charcoal");
  await expect.poll(() => titleColor(page)).toBe(CHARCOAL_TITLE_COLOR);

  await page.reload();

  await expect(page.locator("#theme-select")).toHaveValue("charcoal");
  await expect.poll(() => titleColor(page)).toBe(CHARCOAL_TITLE_COLOR);
});

test("an unknown stored theme falls back to the default", async ({ page }) => {
  await page.evaluate(() => localStorage.setItem("player.theme", "solarized"));

  await page.reload();

  await expect(themeLink(page)).toHaveAttribute("href", "/themes/theme-default.css");
  await expect(page.locator("#theme-select")).toHaveValue("default");
  await expect.poll(() => titleColor(page)).toBe(DEFAULT_TITLE_COLOR);
});

test("the charcoal theme fits a short window with the equalizer open", async ({ page }) => {
  await page.selectOption("#theme-select", "charcoal");
  await page.setViewportSize({ width: 1280, height: 500 });
  await page.click("#equalizer-toggle");
  await expect(page.locator("#equalizer-panel")).toBeVisible();

  await expect.poll(() => pageOverflow(page)).toBe(0);
});
