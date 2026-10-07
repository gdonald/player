import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { currentQueueRow, logIn, mp3Id, mp3Row, position, resetData, sql, songLength, transport, waitUntilPlaying } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

function storedResume(page: Page) {
  return page.evaluate(() => window.localStorage.getItem("player.resume"));
}

async function queueEntryId(title: string) {
  await sql("INSERT INTO queued_mp3s (mp3_id, position) VALUES ($1, 1)", [await mp3Id(title)]);
  const rows = await sql<{ id: number }>("SELECT id FROM queued_mp3s");

  return rows[0].id;
}

test("after a reload the current song comes back paused where it was", async ({ page }) => {
  await logIn(page);
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await waitUntilPlaying(page);
  await page.click("#player-pause");
  await page.locator("#player-seek").fill("30");
  await expect.poll(() => storedResume(page)).toMatch(/^\d+;30\.0$/);

  await page.reload();

  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect(page.locator("#player-title")).toHaveText(/Other Band - Encore/);
  await expect.poll(() => transport(page)).toBe("paused");
  await songLength(page);
  expect(await position(page)).toBeCloseTo(30, 0);
});

test("play after a reload continues from the saved position", async ({ page }) => {
  const entryId = await queueEntryId("Encore");
  await page.addInitScript((saved) => window.localStorage.setItem("player.resume", saved), `${entryId};20.0`);
  await logIn(page);
  await expect.poll(() => transport(page)).toBe("paused");

  await page.click("#player-play");

  await expect.poll(() => transport(page)).toBe("playing");
  await expect.poll(() => position(page)).toBeGreaterThan(20);
});

test("a saved entry that left the queue is not restored", async ({ page }) => {
  await page.addInitScript(() => {
    if (window.localStorage.getItem("player.resume") === null) {
      window.localStorage.setItem("player.resume", "999999;5.0");
    }
  });
  await logIn(page);

  await expect.poll(() => storedResume(page)).toBe("");
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
});

test("a browser that cannot play audio restores nothing", async ({ page }) => {
  const entryId = await queueEntryId("Encore");
  await page.addInitScript((saved) => {
    window.AudioContext = function () {
      throw new Error("Web Audio is unavailable");
    } as unknown as typeof AudioContext;
    if (window.localStorage.getItem("player.resume") === null) {
      window.localStorage.setItem("player.resume", saved);
    }
  }, `${entryId};5.0`);
  await logIn(page);

  await expect.poll(() => storedResume(page)).toBe("");
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
});
