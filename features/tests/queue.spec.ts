import { expect, test } from "@playwright/test";
import { currentQueueRow, endTrack, logIn, mp3Id, mp3Row, queueTitles, resetData, sql } from "./helpers";

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
  await expect(page.locator("audio")).toHaveAttribute("src", `/api/mp3s/${await mp3Id("Encore")}/play`);
  await expect(page).toHaveTitle("Other Band: Encore");
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
  await expect(page.locator("audio")).toHaveCount(0);
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

test("deleting the current entry plays the next one", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue .queue-delete").first().click();

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
  await expect(currentQueueRow(page)).toContainText("Opening Song");
});

test("deleting another entry keeps the current one playing", async ({ page }) => {
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await page.locator("#queue .queue-delete").last().click();

  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
});
