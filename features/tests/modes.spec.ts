import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { currentQueueRow, endTrack, logIn, mp3Row, queueTitles, resetData } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

async function enqueue(page: Page, title: string) {
  const before = (await queueTitles(page)).length;
  await mp3Row(page, title).locator(".play-mp3").click();
  await expect.poll(async () => (await queueTitles(page)).length).toBe(before + 1);
}

function audioTime(page: Page) {
  return page.locator("#player-audio").evaluate((audio: HTMLAudioElement) => audio.currentTime);
}

async function seekTo(page: Page, seconds: number) {
  await expect(page.locator("#player-duration")).toHaveText(/^1:0\d$/);
  await page.locator("#player-seek").fill(String(seconds));
  await expect.poll(() => audioTime(page)).toBeGreaterThanOrEqual(seconds);
}

function modeButton(page: Page, mode: "play" | "loop-one" | "loop-all") {
  return page.locator(`#mode-${mode}`);
}

test("play through is on by default and only one mode is on at a time", async ({ page }) => {
  await expect(modeButton(page, "play")).toHaveAttribute("aria-pressed", "true");

  await modeButton(page, "loop-one").click();

  await expect(modeButton(page, "loop-one")).toHaveAttribute("aria-pressed", "true");
  await expect(modeButton(page, "play")).toHaveAttribute("aria-pressed", "false");
  await expect(modeButton(page, "loop-all")).toHaveAttribute("aria-pressed", "false");
});

test("the mode buttons show icons only", async ({ page }) => {
  for (const mode of ["play", "loop-one", "loop-all"] as const) {
    await expect(modeButton(page, mode)).toHaveText("");
    await expect(modeButton(page, mode).locator("i")).toHaveCount(1);
  }
});

test("the mode is kept across a reload", async ({ page }) => {
  await modeButton(page, "loop-all").click();

  await page.reload();

  await expect(modeButton(page, "loop-all")).toHaveAttribute("aria-pressed", "true");
});

test("loop one starts the same song over and keeps every song", async ({ page }) => {
  await modeButton(page, "loop-one").click();
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await seekTo(page, 30);

  await endTrack(page);

  await expect.poll(() => audioTime(page)).toBeLessThan(5);
  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("loop all plays the next song and keeps the finished one", async ({ page }) => {
  await modeButton(page, "loop-all").click();
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await endTrack(page);

  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("loop all plays the first song after the last", async ({ page }) => {
  await modeButton(page, "loop-all").click();
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await endTrack(page);
  await expect(currentQueueRow(page)).toContainText("Opening Song");

  await endTrack(page);

  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("loop all starts a song queued twice over when it follows itself", async ({ page }) => {
  await modeButton(page, "loop-all").click();
  await enqueue(page, "Encore");
  await enqueue(page, "Encore");
  await seekTo(page, 30);

  await endTrack(page);

  await expect(page.locator("#queue tr").nth(1)).toHaveClass(/table-primary/);
  await expect.poll(() => audioTime(page)).toBeLessThan(5);
});

test("next in a loop mode moves on, wraps, and keeps every song", async ({ page }) => {
  await modeButton(page, "loop-one").click();
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.click("#player-next");
  await expect(currentQueueRow(page)).toContainText("Opening Song");

  await page.click("#player-next");
  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("next in play-through mode removes the current song", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.click("#player-next");

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
});
