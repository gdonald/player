import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { logIn, mp3Row, position, queueTitles, resetData, transport } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

function panel(page: Page) {
  return page.locator("#equalizer-panel");
}

async function openPanel(page: Page) {
  await page.click("#equalizer-toggle");
  await expect(panel(page)).toBeVisible();
}

function gainLabels(page: Page) {
  return page.locator(".equalizer-gain");
}

function storedSettings(page: Page) {
  return page.evaluate(() => window.localStorage.getItem("player.equalizer"));
}

test("the equalizer button opens and closes the panel", async ({ page }) => {
  await openPanel(page);
  await expect(page.locator("#equalizer-toggle")).toHaveAttribute("aria-expanded", "true");

  await page.click("#equalizer-toggle");

  await expect(panel(page)).toHaveCount(0);
});

test("the open or closed panel is kept across a reload", async ({ page }) => {
  await openPanel(page);
  await page.reload();
  await expect(panel(page)).toBeVisible();

  await page.click("#equalizer-toggle");
  await page.reload();
  await expect(page.locator(".library-nav")).toBeVisible();
  await expect(panel(page)).toHaveCount(0);
});

test("the equalizer sits under the playlist", async ({ page }) => {
  await openPanel(page);

  const playlist = await page.locator(".playlist").boundingBox();
  const equalizer = await page.locator(".equalizer").boundingBox();

  expect(equalizer!.y).toBeGreaterThanOrEqual(playlist!.y + playlist!.height - 1);
  expect(equalizer!.x).toBeCloseTo(playlist!.x, 0);
});

test("the panel starts off and flat with ten bands", async ({ page }) => {
  await openPanel(page);

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator("#equalizer-preset")).toHaveValue("Flat");
  await expect(page.locator(".equalizer-frequency")).toHaveText([
    "PREAMP", "32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
  ]);
  await expect(gainLabels(page)).toHaveText(Array(11).fill("0"));
});

test("choosing a preset sets every band and turns the equalizer on", async ({ page }) => {
  await openPanel(page);

  await page.selectOption("#equalizer-preset", "Bass Boost");

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect(gainLabels(page)).toHaveText(["0", "+6", "+5", "+4", "+2", "0", "0", "0", "0", "0", "0"]);
  await expect(page.locator("#equalizer-on")).toHaveClass(/winamp-lit/);
});

test("moving a band makes the preset custom and turns the equalizer on", async ({ page }) => {
  await openPanel(page);

  await page.locator("#equalizer-band-5").fill("4.5");

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#equalizer-preset")).toHaveValue("Custom");
  await expect(gainLabels(page).nth(6)).toHaveText("+4.5");
});

test("reset flattens the bands and the preamp", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Rock");
  await page.locator("#equalizer-preamp").fill("-6");

  await page.click("#equalizer-reset");

  await expect(page.locator("#equalizer-preset")).toHaveValue("Flat");
  await expect(gainLabels(page)).toHaveText(Array(11).fill("0"));
});

test("the preamp slider sets the preamp and turns the equalizer on", async ({ page }) => {
  await openPanel(page);

  await page.locator("#equalizer-preamp").fill("-6");

  await expect(gainLabels(page).first()).toHaveText("-6");
  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect.poll(() => storedSettings(page)).toBe("on;-6;0,0,0,0,0,0,0,0,0,0");
});

test("the curve follows the bands", async ({ page }) => {
  await openPanel(page);
  const line = page.locator(".equalizer-curve-line");
  await expect(line).toHaveAttribute("points", "0,10 10,10 20,10 30,10 40,10 50,10 60,10 70,10 80,10 90,10");

  await page.locator("#equalizer-band-0").fill("12");

  await expect(line).toHaveAttribute("points", /^0,0 10,10 /);
});

test("turning the equalizer off keeps the bands", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Vocal");

  await page.click("#equalizer-on");

  await expect(page.locator("#equalizer-preset")).toHaveValue("Vocal");
  await expect(page.locator(".equalizer-bands")).toHaveClass(/equalizer-bands-off/);
  await expect(page.locator("#equalizer-on")).not.toHaveClass(/winamp-lit/);
});

test("settings are kept across a reload", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Treble Boost");
  await expect.poll(() => storedSettings(page)).toBe("on;0;0,0,0,0,0,1,2,4,5,6");

  await page.reload();
  await expect(panel(page)).toBeVisible();

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#equalizer-preset")).toHaveValue("Treble Boost");
});

test("playback keeps going through the equalizer", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Bass Boost");

  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);

  await expect.poll(() => position(page)).toBeGreaterThan(1);
  await expect.poll(() => transport(page)).toBe("playing");
});

test("the panel shows under the playlist", async ({ page }, testInfo) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Rock");
  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");

  await page.screenshot({ path: testInfo.outputPath("equalizer.png"), animations: "disabled" });
});
