import { Page, expect, test } from "@playwright/test";
import { logIn, mp3Row, queueTitles, resetData } from "./helpers";

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

/// Swaps the playing element's source for a generated 440 Hz tone, so the
/// analyzer has sound to show. The demo library's MP3s are silent.
async function playTone(page: Page) {
  await page.locator("#player-audio").evaluate(async (audio: HTMLAudioElement) => {
    const sampleRate = 44100;
    const seconds = 5;
    const samples = sampleRate * seconds;
    const buffer = new ArrayBuffer(44 + samples * 2);
    const view = new DataView(buffer);
    const text = (offset: number, value: string) =>
      [...value].forEach((character, index) => view.setUint8(offset + index, character.charCodeAt(0)));

    text(0, "RIFF");
    view.setUint32(4, 36 + samples * 2, true);
    text(8, "WAVE");
    text(12, "fmt ");
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);
    view.setUint16(22, 1, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 2, true);
    view.setUint16(32, 2, true);
    view.setUint16(34, 16, true);
    text(36, "data");
    view.setUint32(40, samples * 2, true);
    for (let index = 0; index < samples; index += 1) {
      view.setInt16(44 + index * 2, Math.sin((2 * Math.PI * 440 * index) / sampleRate) * 20000, true);
    }

    audio.src = URL.createObjectURL(new Blob([buffer], { type: "audio/wav" }));
    await audio.play();
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
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect(page.locator("#player-audio")).toHaveCount(1);

  await playTone(page);

  await expect.poll(() => barPixels(page), { timeout: 10_000 }).toBeGreaterThan(50);

  await page.locator(".queue-wrapper").screenshot({ path: testInfo.outputPath("analyzer.png") });
});

test("the analyzer still shows bars through a boosted equalizer", async ({ page }) => {
  await page.click("#equalizer-toggle");
  await page.selectOption("#equalizer-preset", "Bass Boost");
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect(page.locator("#player-audio")).toHaveCount(1);

  await playTone(page);

  await expect.poll(() => barPixels(page), { timeout: 10_000 }).toBeGreaterThan(50);
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
