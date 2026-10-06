import { expect, test } from "./fixtures";
import {
  createPlaylist,
  currentQueueRow,
  endTrack,
  logIn,
  mp3Id,
  mp3Row,
  position,
  queueTitles,
  resetData,
  songLength,
  sql,
  transport,
  waitUntilPlaying,
} from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

async function enqueue(page: import("@playwright/test").Page, title: string) {
  const before = (await queueTitles(page)).length;
  await mp3Row(page, title).locator(".play-mp3").click();
  await expect.poll(async () => (await queueTitles(page)).length).toBe(before + 1);
}

test("enqueuing a song starts playing it", async ({ page }) => {
  await enqueue(page, "Encore");

  await expect(currentQueueRow(page)).toContainText("Encore");
  await waitUntilPlaying(page);
  await expect(page).toHaveTitle("Other Band: Encore");
});

test("the clear button is disabled while the playlist is empty", async ({ page }) => {
  await expect(page.locator("#queue-clear")).toBeDisabled();
});

test("clear empties the playlist and stops playback", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await expect(currentQueueRow(page)).toContainText("Encore");

  await page.click("#queue-clear");

  await expect.poll(() => queueTitles(page)).toEqual([]);
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect(page).toHaveTitle("Player");
  await expect(page.locator("#queue-clear")).toBeDisabled();
  expect(await sql("SELECT 1 FROM queued_mp3s")).toHaveLength(0);
});

test("a cleared playlist stays empty after a reload", async ({ page }) => {
  await enqueue(page, "Encore");
  await page.click("#queue-clear");
  await expect.poll(() => queueTitles(page)).toEqual([]);

  await page.reload();

  await expect(page.locator(".library-nav")).toBeVisible();
  await expect.poll(() => queueTitles(page)).toEqual([]);
});

test("a second song waits in the queue", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("the end of a track plays the next one", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await endTrack(page);

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await expect(page).toHaveTitle("The Testers: Opening Song");
});

test("the end of the last track stops playback", async ({ page }) => {
  await enqueue(page, "Encore");

  await endTrack(page);

  await expect.poll(() => queueTitles(page)).toEqual([]);
  await expect.poll(() => transport(page)).toBe("stopped");
  await expect(page).toHaveTitle("Player");
});

test("the play button starts a stopped queue", async ({ page }) => {
  await sql("INSERT INTO queued_mp3s (mp3_id, position) VALUES ($1, 1), ($2, 2)", [
    await mp3Id("Second Song"),
    await mp3Id("Encore"),
  ]);
  await page.reload();

  await expect.poll(() => queueTitles(page)).toEqual(["Second Song", "Encore"]);
  await page.click("#queue-play");

  await expect(currentQueueRow(page)).toContainText("Second Song");
  await expect(page.locator("#queue-play")).toHaveCount(0);
});

test("removing the playing entry stops playback and keeps the rest", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue .queue-delete").first().click();

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect.poll(() => transport(page)).toBe("stopped");
  await expect(currentQueueRow(page)).toHaveCount(0);
});

test("double-clicking a remove button does not play an entry", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue .queue-delete").last().dblclick();

  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
});

test("double-clicking a playlist row plays that entry and keeps the others", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue tr", { hasText: "Opening Song" }).dblclick();

  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("double-clicking the playing row starts it over", async ({ page }) => {
  await enqueue(page, "Encore");
  await songLength(page);
  await page.locator("#player-seek").fill("30");
  await expect.poll(() => position(page)).toBeGreaterThanOrEqual(30);

  await currentQueueRow(page).dblclick();

  await expect.poll(() => position(page)).toBeLessThan(5);
});

test("enqueuing after stop plays the newly added song", async ({ page }) => {
  await enqueue(page, "Encore");
  await page.click("#player-stop");
  await expect(page.locator("#player-state")).toHaveClass(/bi-stop-fill/);

  await enqueue(page, "Opening Song");

  await expect(currentQueueRow(page)).toContainText("Opening Song");
});

test("enqueuing with older songs queued plays the first newly added song", async ({ page, request }) => {
  await sql("INSERT INTO queued_mp3s (mp3_id, position) VALUES ($1, 1)", [await mp3Id("Encore")]);
  await createPlaylist(request, "Mix", ["Opening Song", "Second Song"]);
  await page.reload();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);

  await page.locator(".library-button", { hasText: "Playlists" }).click();
  await page.locator("#playlists tr", { hasText: "Mix" }).locator(".enqueue-playlist").click();

  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song", "Second Song"]);
  await expect(currentQueueRow(page)).toContainText("Opening Song");
});

test("enqueuing while a song is paused partway keeps it", async ({ page }) => {
  await enqueue(page, "Encore");
  await expect(page.locator("#player-duration")).toHaveText(/^1:0\d$/);
  await page.locator("#player-seek").fill("20");
  await page.click("#player-pause");
  await expect(page.locator("#player-state")).toHaveClass(/bi-pause-fill/);

  await enqueue(page, "Opening Song");

  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect(page.locator("#player-state")).toHaveClass(/bi-pause-fill/);
});

test("deleting another entry keeps the current one playing", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue .queue-delete").last().click();

  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
});

test("played songs collect in Recently Played, newest first", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await endTrack(page);
  await expect(currentQueueRow(page)).toContainText("Opening Song");

  await page.locator(".library-button", { hasText: "Playlists" }).click();
  await page.locator("#playlists tr", { hasText: "Recently Played" }).locator(".edit-playlist").click();

  await expect(page.locator("#playlist-mp3s .play-mp3")).toHaveText(["Opening Song", "Encore"]);
});
