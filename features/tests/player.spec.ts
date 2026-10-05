import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { logIn, mp3Id, mp3Row, queueTitles, resetData, sql } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

async function play(page: Page, title: string) {
  const before = (await queueTitles(page)).length;
  await mp3Row(page, title).locator(".play-mp3").click();
  await expect.poll(async () => (await queueTitles(page)).length).toBe(before + 1);
}

function audioProperty(page: Page, property: "paused" | "currentTime" | "volume" | "muted") {
  return page.locator("#player-audio").evaluate((audio: HTMLAudioElement, name) => audio[name], property);
}

async function waitForLength(page: Page) {
  await expect(page.locator("#player-duration")).toHaveText(/^1:0\d$/);
}

async function waitUntilPlaying(page: Page) {
  await expect.poll(() => audioProperty(page, "paused")).toBe(false);
}

test("with nothing playing the controls are disabled", async ({ page }) => {
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect(page.locator("#player-play")).toBeDisabled();
  await expect(page.locator("#player-pause")).toBeDisabled();
  await expect(page.locator("#player-stop")).toBeDisabled();
  await expect(page.locator("#player-next")).toBeDisabled();
  await expect(page.locator("#player-restart")).toBeDisabled();
  await expect(page.locator("#player-seek")).toBeDisabled();
  await expect(page.locator("#player-position")).toHaveText("0:00");
  await expect(page.locator("#player-duration")).toHaveText("0:00");
  await expect(page.locator("#player-state")).toHaveClass(/bi-stop-fill/);
  await expect(page.locator("audio[controls]")).toHaveCount(0);
});

test("the playing track shows its number, artist, title, length, and album", async ({ page }, testInfo) => {
  await play(page, "Encore");

  await expect(page.locator("#player-title")).toHaveText(/^1\. Other Band - Encore \(1:0\d\)$/);
  await expect(page.locator("#player-byline")).toHaveText("Live");
  await waitForLength(page);
  await expect(page.locator("#player-state")).toHaveClass(/bi-play-fill/);

  await page.screenshot({ path: testInfo.outputPath("page.png") });
});

test("pause pauses and resumes", async ({ page }) => {
  await play(page, "Encore");
  await waitUntilPlaying(page);

  await page.click("#player-pause");

  await expect.poll(() => audioProperty(page, "paused")).toBe(true);
  await expect(page.locator("#player-state")).toHaveClass(/bi-pause-fill/);

  await page.click("#player-pause");

  await waitUntilPlaying(page);
  await expect(page.locator("#player-state")).toHaveClass(/bi-play-fill/);
});

test("play resumes a paused track", async ({ page }) => {
  await play(page, "Encore");
  await waitUntilPlaying(page);
  await page.click("#player-pause");
  await expect.poll(() => audioProperty(page, "paused")).toBe(true);

  await page.click("#player-play");

  await waitUntilPlaying(page);
});

test("stop pauses and returns to the start", async ({ page }) => {
  await play(page, "Encore");
  await waitForLength(page);
  await page.locator("#player-seek").fill("30");
  await expect.poll(() => audioProperty(page, "currentTime")).toBeGreaterThanOrEqual(30);

  await page.click("#player-stop");

  await expect.poll(() => audioProperty(page, "paused")).toBe(true);
  await expect.poll(() => audioProperty(page, "currentTime")).toBe(0);
  await expect(page.locator("#player-state")).toHaveClass(/bi-stop-fill/);
  await expect(page.locator("#player-position")).toHaveText("0:00");
});

test("play on a playing track starts it over", async ({ page }) => {
  await play(page, "Encore");
  await waitForLength(page);
  await page.locator("#player-seek").fill("30");
  await expect.poll(() => audioProperty(page, "currentTime")).toBeGreaterThanOrEqual(30);

  await page.click("#player-play");

  await expect.poll(() => audioProperty(page, "currentTime")).toBeLessThan(5);
  await waitUntilPlaying(page);
});

test("the space bar pauses and resumes after clicking a title", async ({ page }) => {
  await play(page, "Encore");
  await waitUntilPlaying(page);

  await page.keyboard.press(" ");
  await expect.poll(() => audioProperty(page, "paused")).toBe(true);

  await page.keyboard.press(" ");
  await waitUntilPlaying(page);
});

test("the space bar types in the search box", async ({ page }) => {
  await play(page, "Encore");
  await waitUntilPlaying(page);

  await page.locator("#search").press(" ");

  await expect.poll(() => audioProperty(page, "paused")).toBe(false);
});

test("dragging the seek bar moves the playback position", async ({ page }) => {
  await play(page, "Encore");
  await waitForLength(page);

  await page.locator("#player-seek").fill("30");

  await expect.poll(() => audioProperty(page, "currentTime")).toBeGreaterThanOrEqual(30);
  await expect(page.locator("#player-position")).toHaveText(/^0:3\d$/);
});

test("the length display counts down the time left", async ({ page }) => {
  await play(page, "Encore");
  await waitForLength(page);

  await page.locator("#player-seek").fill("30");

  await expect.poll(() => audioProperty(page, "currentTime")).toBeGreaterThanOrEqual(30);
  await expect(page.locator("#player-duration")).toHaveText(/^0:3\d$/);
});

test("restart returns to the start of the track", async ({ page }) => {
  await play(page, "Encore");
  await waitForLength(page);
  await page.locator("#player-seek").fill("30");
  await expect.poll(() => audioProperty(page, "currentTime")).toBeGreaterThanOrEqual(30);

  await page.click("#player-restart");

  await expect.poll(() => audioProperty(page, "currentTime")).toBeLessThan(5);
});

test("next plays the following entry", async ({ page }) => {
  await play(page, "Encore");
  await play(page, "Opening Song");

  await page.click("#player-next");

  await expect(page.locator("#player-title")).toHaveText(/^1\. The Testers - Opening Song \(1:0\d\)$/);
  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
});

test("play starts a stopped queue", async ({ page }) => {
  await sql("INSERT INTO queued_mp3s (mp3_id, position) VALUES ($1, 1)", [await mp3Id("Second Song")]);
  await page.reload();
  await expect.poll(() => queueTitles(page)).toEqual(["Second Song"]);
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");

  await page.click("#player-play");

  await expect(page.locator("#player-title")).toHaveText(/Second Song/);
  await waitUntilPlaying(page);
});

test("play is disabled with an empty queue", async ({ page }) => {
  await expect(page.locator("#player-play")).toBeDisabled();
});

test("next on the last entry stops playback", async ({ page }) => {
  await play(page, "Encore");

  await page.click("#player-next");

  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect(page.locator("#player-audio")).toHaveCount(0);
});

test("the volume slider sets the volume and is remembered", async ({ page }) => {
  await play(page, "Encore");

  await page.locator("#player-volume").fill("0.3");

  await expect.poll(() => audioProperty(page, "volume")).toBeCloseTo(0.3);
  await expect(page.locator("#player-mute i")).toHaveClass("bi-volume-down-fill");

  await page.reload();
  await expect(page.locator("#player-volume")).toHaveValue("0.3");
  await page.click("#queue-play");

  await expect.poll(() => audioProperty(page, "volume")).toBeCloseTo(0.3);
});

test("the mute button mutes and unmutes", async ({ page }) => {
  await play(page, "Encore");

  await page.click("#player-mute");

  await expect.poll(() => audioProperty(page, "muted")).toBe(true);
  await expect(page.locator("#player-mute i")).toHaveClass("bi-volume-mute-fill");

  await page.click("#player-mute");

  await expect.poll(() => audioProperty(page, "muted")).toBe(false);
  await expect(page.locator("#player-mute i")).toHaveClass("bi-volume-up-fill");
});

test("a title that fits stays still", async ({ page }) => {
  await play(page, "Encore");

  await expect(page.locator("#player-title")).toHaveText(/Encore/);
  await expect(page.locator("#player-title")).not.toHaveClass(/player-marquee-scrolling/);
});

test("a title wider than its box slides by its overflow", async ({ page }) => {
  await sql("UPDATE mp3s SET title = $1 WHERE title = 'Encore'", [
    "Encore " + "and a much longer title that cannot fit in the box ".repeat(4),
  ]);
  await page.reload();
  await play(page, "Encore and");

  const title = page.locator("#player-title");
  await expect(title).toHaveClass(/player-marquee-scrolling/);
  await expect(title).toHaveAttribute("style", /--marquee-shift: -\d+px/);
});

test("the space bar with nothing playing does nothing", async ({ page }) => {
  await page.locator("body").press("Space");

  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect(page.locator("#player-audio")).toHaveCount(0);
});

test("keys other than the space bar leave playback alone", async ({ page }) => {
  await play(page, "Encore");
  await waitUntilPlaying(page);

  await page.locator("body").press("a");

  await expect.poll(() => audioProperty(page, "paused")).toBe(false);
});
