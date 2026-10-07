import { expect, test } from "./fixtures";
import { alert, createPlaylist, currentQueueRow, logIn, menu, queueTitles, resetData, transport, waitUntilPlaying } from "./helpers";

let playlistId: number;

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  playlistId = await createPlaylist(request, "Mix", ["Opening Song", "Second Song", "Encore"]);
  await logIn(page);
  await menu(page, "Playlists").click();
});

function entryTitles(page: import("@playwright/test").Page) {
  return page.locator("#playlist-mp3s .play-mp3");
}

async function openMix(page: import("@playwright/test").Page) {
  await page.locator(`#playlist-${playlistId} .edit-playlist`).click();
  await expect(page.locator("#name")).toHaveValue("Mix");
}

test("playlists list Recently Played first with mp3 counts", async ({ page }) => {
  await expect(page.locator("#playlists tbody tr td:first-child")).toHaveText(["Recently Played", "Mix"]);
  await expect(page.locator(`#playlist-${playlistId} td`).nth(1)).toHaveText("3");
  await expect(page.locator(".enqueue-playlist")).toHaveCount(1);
});

test("a playlist is renamed", async ({ page }) => {
  await openMix(page);
  await page.fill("#name", "Mixed");
  await page.click("#save");

  await expect(alert(page)).toContainText("Playlist updated");
  await expect(page.locator(".breadcrumb")).toContainText("Mixed");
});

test("a taken name shows an error", async ({ page }) => {
  await openMix(page);
  await page.fill("#name", "Recently Played");
  await page.click("#save");

  await expect(page.locator(".error")).toHaveText("has already been taken");
});

test("entries are ordered by track", async ({ page }) => {
  await openMix(page);

  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);
});

function handle(page: import("@playwright/test").Page, title: string) {
  return page.locator("#playlist-mp3s tbody tr", { hasText: title }).locator(".drag-handle");
}

test("dragging an entry's handle down moves it below the row it is dropped on", async ({ page }) => {
  await openMix(page);
  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);

  await handle(page, "Opening Song").dragTo(handle(page, "Second Song"));

  await expect(entryTitles(page)).toHaveText(["Encore", "Second Song", "Opening Song"]);
});

test("dragging an entry's handle up moves it above the row it is dropped on", async ({ page }) => {
  await openMix(page);
  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);

  await handle(page, "Second Song").dragTo(handle(page, "Encore"));

  await expect(entryTitles(page)).toHaveText(["Opening Song", "Second Song", "Encore"]);
});

test("a dragged order is kept after a reload", async ({ page }) => {
  await openMix(page);
  await handle(page, "Second Song").dragTo(handle(page, "Opening Song"));
  await expect(entryTitles(page)).toHaveText(["Second Song", "Opening Song", "Encore"]);

  await page.reload();
  await menu(page, "Playlists").click();
  await openMix(page);

  await expect(entryTitles(page)).toHaveText(["Second Song", "Opening Song", "Encore"]);
});

test("dropping an entry on itself leaves the order alone", async ({ page }) => {
  await openMix(page);

  await handle(page, "Encore").dragTo(page.locator("#playlist-mp3s tbody tr", { hasText: "Encore" }).locator(".col-track"));

  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);
});

test("the row under a dragged entry shows where it will land", async ({ page }) => {
  await openMix(page);

  await handle(page, "Opening Song").hover();
  await page.mouse.down();
  const target = (await handle(page, "Second Song").boundingBox())!;
  // Chromium only fires dragover after more than one move over the target.
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 5 });
  await page.mouse.move(target.x + target.width / 2 + 1, target.y + target.height / 2);

  await expect(page.locator("#playlist-mp3s tbody tr", { hasText: "Second Song" })).toHaveClass(/drop-below/);

  await page.mouse.up();
});

test("an entry is removed", async ({ page }) => {
  await openMix(page);
  await page.locator(".delete-entry").first().click();

  await expect(entryTitles(page)).toHaveText(["Encore", "Second Song"]);
});

test("an album link in a playlist searches the library", async ({ page }) => {
  await openMix(page);
  await page.locator("#playlist-mp3s .search-album", { hasText: "Live" }).click();

  await expect(page.locator("#search")).toHaveValue('album:"Live"');
  await expect(page.locator("#mp3s .play-mp3")).toHaveText(["Encore"]);
});

test("a playlist is deleted after confirming", async ({ page }) => {
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator(`#playlist-${playlistId} .delete-playlist`).click();

  await expect(alert(page)).toContainText("Playlist deleted");
  await expect(page.locator(`#playlist-${playlistId}`)).toHaveCount(0);
});

test("dismissing the confirmation keeps the playlist", async ({ page }) => {
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.locator(`#playlist-${playlistId} .delete-playlist`).click();

  await expect(page.locator(`#playlist-${playlistId}`)).toHaveCount(1);
});

test("enqueue from the list queues the playlist and starts it", async ({ page }) => {
  await page.locator(`#playlist-${playlistId} .enqueue-playlist`).click();

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song", "Encore", "Second Song"]);
  await expect(currentQueueRow(page)).toContainText("Opening Song");
});

test("next on an enqueued playlist plays the following song", async ({ page }) => {
  await page.locator(`#playlist-${playlistId} .enqueue-playlist`).click();
  await waitUntilPlaying(page);

  await page.click("#player-next");

  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Second Song"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
  await waitUntilPlaying(page);
});

test("next right after enqueueing a playlist plays the following song", async ({ page }) => {
  await page.locator(`#playlist-${playlistId} .enqueue-playlist`).click();
  await expect(currentQueueRow(page)).toContainText("Opening Song");

  await page.click("#player-next");

  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Second Song"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
  await waitUntilPlaying(page);
});

test("next after a reload plays the following song", async ({ page }) => {
  await page.locator(`#playlist-${playlistId} .enqueue-playlist`).click();
  await waitUntilPlaying(page);
  await page.waitForFunction(() => /^\d+;/.test(window.localStorage.getItem("player.resume") ?? ""));
  await page.reload();
  await expect.poll(() => transport(page)).toBe("paused");

  await page.click("#player-next");

  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Second Song"]);
  await expect(currentQueueRow(page)).toContainText("Encore");
  await waitUntilPlaying(page);
});

test("enqueue from the playlist view queues the playlist", async ({ page }) => {
  await openMix(page);
  await page.click("#enqueue");

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song", "Encore", "Second Song"]);
});

test("the playlist page's breadcrumb returns to the playlists", async ({ page }) => {
  await openMix(page);

  await page.locator(".breadcrumb a", { hasText: "Playlists" }).click();

  await expect(page.locator("#playlists")).toBeVisible();
});

test("clicking an entry's title queues the song", async ({ page }) => {
  await openMix(page);

  await entryTitles(page).filter({ hasText: "Encore" }).click();

  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
});

test("clicking an entry's artist searches for the artist", async ({ page }) => {
  await openMix(page);

  await page.locator("#playlist-mp3s tbody tr", { hasText: "Encore" }).locator(".search-artist").click();

  await expect(page.locator("#search")).toHaveValue('artist:"Other Band"');
});

test("dragging something other than an entry over a row marks nothing", async ({ page }) => {
  await openMix(page);
  const row = page.locator("#playlist-mp3s tbody tr", { hasText: "Encore" });

  await row.dispatchEvent("dragover");

  await expect(row).toHaveClass("align-middle");
});

test("a drag started without drag data still moves the entry", async ({ page }) => {
  await openMix(page);
  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);

  await handle(page, "Second Song").dispatchEvent("dragstart");
  const target = page.locator("#playlist-mp3s tbody tr", { hasText: "Opening Song" });
  await target.dispatchEvent("dragover");
  await target.dispatchEvent("drop");

  await expect(entryTitles(page)).toHaveText(["Second Song", "Opening Song", "Encore"]);
});

test("deleting a playlist lowers the Playlists count", async ({ page }) => {
  const playlistsCount = menu(page, "Playlists").locator(".library-count");
  await expect(playlistsCount).toHaveText("2");
  page.once("dialog", (dialog) => dialog.accept());

  await page.locator(`#playlist-${playlistId} .delete-playlist`).click();

  await expect(alert(page)).toContainText("Playlist deleted");
  await expect(playlistsCount).toHaveText("1");
});
