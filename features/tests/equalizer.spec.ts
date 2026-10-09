import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { logIn, mp3Row, position, queueTitles, resetData, storedSetting, transport } from "./helpers";

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

function storedSettings() {
  return storedSetting("equalizer");
}

test("the equalizer button opens and closes the panel", async ({ page }) => {
  await openPanel(page);
  await expect(page.locator("#equalizer-toggle")).toHaveAttribute("aria-expanded", "true");

  await page.click("#equalizer-toggle");

  await expect(panel(page)).toHaveCount(0);
});

test("the open or closed panel is kept across a reload", async ({ page }) => {
  await openPanel(page);
  await expect.poll(() => storedSetting("equalizer_open")).toBe("true");
  await page.reload();
  await expect(panel(page)).toBeVisible();

  await page.click("#equalizer-toggle");
  await expect.poll(() => storedSetting("equalizer_open")).toBe("false");
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
  await expect.poll(() => storedSettings()).toBe("on;-6;0,0,0,0,0,0,0,0,0,0");
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
  await expect.poll(() => storedSettings()).toBe("on;0;0,0,0,0,0,1,2,4,5,6");

  await page.reload();
  await expect(panel(page)).toBeVisible();

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#equalizer-preset")).toHaveValue("Treble Boost");
});

function storedPresets() {
  return storedSetting("equalizer_presets");
}

async function savePreset(page: Page, name: string) {
  await page.click("#equalizer-save");
  await page.fill("#preset-name", name);
  await page.click("#preset-save");
}

test("the save button opens a dialog with the name field focused", async ({ page }) => {
  await openPanel(page);

  await page.click("#equalizer-save");

  await expect(page.locator("#preset-dialog .winamp-titlebar")).toHaveText("SAVE PRESET");
  await expect(page.locator("#preset-name")).toBeFocused();
});

test("saving a preset keeps the bands and preamp under its name", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Rock");
  await page.locator("#equalizer-preamp").fill("-3");

  await savePreset(page, "  Late Night ");

  await expect(page.locator("#preset-dialog")).toHaveCount(0);
  await expect(page.locator("#equalizer-preset")).toHaveValue("Late Night");
  await expect.poll(() => storedPresets()).toBe("-3;5,4,2,-1,-2,-1,1,3,4,5;Late Night");
});

test("pressing Enter in the name field saves the preset", async ({ page }) => {
  await openPanel(page);
  await page.locator("#equalizer-band-0").fill("6");
  await page.click("#equalizer-save");

  await page.fill("#preset-name", "Low End");
  await page.press("#preset-name", "Enter");

  await expect(page.locator("#preset-dialog")).toHaveCount(0);
  await expect(page.locator("#equalizer-preset")).toHaveValue("Low End");
});

test("cancel and Escape close the dialog without saving", async ({ page }) => {
  await openPanel(page);
  await page.click("#equalizer-save");
  await page.fill("#preset-name", "Unsaved");
  await page.press("#preset-name", "a");

  await page.click("#preset-cancel");
  await expect(page.locator("#preset-dialog")).toHaveCount(0);

  await page.click("#equalizer-save");
  await expect(page.locator("#preset-name")).toHaveValue("");
  await page.press("#preset-name", "Escape");

  await expect(page.locator("#preset-dialog")).toHaveCount(0);
  expect(await storedPresets()).toBeNull();
});

test("a blank or built-in name is refused with a message", async ({ page }) => {
  await openPanel(page);
  await page.click("#equalizer-save");

  await page.click("#preset-save");
  await expect(page.locator("#preset-error")).toHaveText("Enter a name");

  await page.fill("#preset-name", "Rock");
  await page.click("#preset-save");
  await expect(page.locator("#preset-error")).toHaveText("A built-in preset has that name");
  await expect(page.locator("#preset-dialog")).toBeVisible();
});

test("choosing a saved preset sets its bands and preamp", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Vocal");
  await page.locator("#equalizer-preamp").fill("-4");
  await savePreset(page, "Talk");
  await page.click("#equalizer-reset");
  await page.click("#equalizer-on");

  await page.selectOption("#equalizer-preset", "Talk");

  await expect(gainLabels(page)).toHaveText(["-4", "-2", "-2", "-1", "0", "+2", "+4", "+4", "+3", "+1", "0"]);
  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
});

test("saving under a saved name replaces that preset", async ({ page }) => {
  await openPanel(page);
  await savePreset(page, "Mine");
  await page.locator("#equalizer-band-9").fill("3");

  await savePreset(page, "Mine");

  await expect(page.locator("#equalizer-preset option", { hasText: "Mine" })).toHaveCount(1);
  await expect.poll(() => storedPresets()).toBe("0;0,0,0,0,0,0,0,0,0,3;Mine");
});

test("saved presets are kept across a reload", async ({ page }) => {
  await openPanel(page);
  await page.locator("#equalizer-band-2").fill("2");
  await savePreset(page, "Kept");
  await expect.poll(() => storedPresets()).toBe("0;0,0,2,0,0,0,0,0,0,0;Kept");
  await expect.poll(() => storedSettings()).toBe("on;0;0,0,2,0,0,0,0,0,0,0");

  await page.reload();
  await expect(panel(page)).toBeVisible();

  await expect(page.locator("#equalizer-preset option")).toContainText(["Kept"]);
  await expect(page.locator("#equalizer-preset")).toHaveValue("Kept");
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

test("settings and presets follow the user to a browser with nothing stored", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Rock");
  await savePreset(page, "Late Night");
  await expect.poll(() => storedPresets()).toBe("0;5,4,2,-1,-2,-1,1,3,4,5;Late Night");
  await expect.poll(() => storedSettings()).toBe("on;0;5,4,2,-1,-2,-1,1,3,4,5");

  await page.evaluate(() => window.localStorage.clear());
  await page.reload();
  await expect(panel(page)).toBeVisible();

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#equalizer-preset")).toHaveValue("Late Night");
});

test("settings this browser kept before are uploaded when the server has none", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("player.equalizer", "on;-2;0,0,0,0,0,1,2,4,5,6");
    window.localStorage.setItem("player.equalizer.presets", "1;1,1,1,1,1,1,1,1,1,1;Kept");
  });

  await page.reload();

  await expect.poll(() => storedSettings()).toBe("on;-2;0,0,0,0,0,1,2,4,5,6");
  await expect.poll(() => storedPresets()).toBe("1;1,1,1,1,1,1,1,1,1,1;Kept");
});

test("the server's settings win over ones this browser kept before", async ({ page }) => {
  await openPanel(page);
  await page.selectOption("#equalizer-preset", "Jazz");
  await expect.poll(() => storedSettings()).toBe("on;0;3,2,1,2,-1,-1,0,1,2,3");
  await page.evaluate(() => window.localStorage.setItem("player.equalizer", "off;0;0,0,0,0,0,0,0,0,0,0"));

  await page.reload();
  await expect(panel(page)).toBeVisible();

  await expect(page.locator("#equalizer-preset")).toHaveValue("Jazz");
});
