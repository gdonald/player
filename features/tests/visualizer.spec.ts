import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { logIn, mp3Row, queueTitles, resetData, serveTone } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

/// Pixels in the bar colors (green through yellow to red), leaving out the
/// gray peak caps and the black background.
function barPixels(page: Page) {
  return page.locator("#visualizer").evaluate((canvas: HTMLCanvasElement) => {
    const context = canvas.getContext("2d")!;
    const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
    let count = 0;
    for (let index = 0; index < data.length; index += 4) {
      const [red, green, blue] = [data[index], data[index + 1], data[index + 2]];
      if ((green > 120 || red > 120) && blue < 80) {
        count += 1;
      }
    }
    return count;
  });
}

test("the analyzer sits under the equalizer, which sits under the playlist", async ({ page }) => {
  const playlist = await page.locator(".playlist").boundingBox();
  const equalizer = await page.locator(".equalizer").boundingBox();
  const analyzer = await page.locator(".visualizer").boundingBox();

  expect(equalizer!.y).toBeGreaterThanOrEqual(playlist!.y + playlist!.height - 1);
  expect(analyzer!.y).toBeGreaterThanOrEqual(equalizer!.y + equalizer!.height - 1);
});

test("the analyzer shows no bars while nothing plays", async ({ page }) => {
  await expect.poll(() => page.locator("#visualizer").evaluate((canvas: HTMLCanvasElement) => canvas.width)).toBeGreaterThan(0);

  expect(await barPixels(page)).toBe(0);
});

test("the analyzer shows bars for playing sound", async ({ page }, testInfo) => {
  await serveTone(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);

  await expect.poll(() => barPixels(page), { timeout: 10_000 }).toBeGreaterThan(50);

  await page.locator(".queue-wrapper").screenshot({ path: testInfo.outputPath("analyzer.png") });
});

test("the analyzer still shows bars through a boosted equalizer", async ({ page }) => {
  await page.click("#equalizer-toggle");
  await page.selectOption("#equalizer-preset", "Bass Boost");
  await serveTone(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect.poll(() => barPixels(page), { timeout: 10_000 }).toBeGreaterThan(50);
});

/// Pixels in the charcoal theme's slate and blue-gray analyzer colors.
function slatePixels(page: Page) {
  return page.locator("#visualizer").evaluate((canvas: HTMLCanvasElement) => {
    const context = canvas.getContext("2d")!;
    const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
    let count = 0;
    for (let index = 0; index < data.length; index += 4) {
      const [red, green, blue] = [data[index], data[index + 1], data[index + 2]];
      if (red >= 60 && red <= 180 && green >= red + 10 && blue >= red + 10 && Math.abs(green - blue) < 20) {
        count += 1;
      }
    }
    return count;
  });
}

test("the charcoal theme draws the analyzer in slate with no green or red", async ({ page }, testInfo) => {
  await page.selectOption("#theme-select", "charcoal");
  await serveTone(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect.poll(() => slatePixels(page), { timeout: 10_000 }).toBeGreaterThan(50);
  expect(await barPixels(page)).toBe(0);

  await page.locator(".queue-wrapper").screenshot({ path: testInfo.outputPath("charcoal-analyzer.png") });
});

test("the VIS button hides and shows the analyzer, and the choice is kept", async ({ page }) => {
  await expect(page.locator("#visualizer")).toBeVisible();

  await page.click("#visualizer-toggle");

  await expect(page.locator("#visualizer")).toHaveCount(0);
  await expect(page.locator("#visualizer-toggle")).toHaveAttribute("aria-expanded", "false");

  await page.reload();
  await expect(page.locator(".library-nav")).toBeVisible();
  await expect(page.locator("#visualizer")).toHaveCount(0);

  await page.click("#visualizer-toggle");

  await expect(page.locator("#visualizer")).toBeVisible();
});
