import { expect, test } from "@playwright/test";
import { alert, createPlaylist, currentQueueRow, logIn, menu, queueTitles, resetData } from "./helpers";

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

test("entries are ordered by track and can be moved", async ({ page }) => {
  await openMix(page);
  await expect(entryTitles(page)).toHaveText(["Opening Song", "Encore", "Second Song"]);
  await expect(page.locator(".move-up").first()).toHaveClass(/disabled/);
  await expect(page.locator(".move-down").last()).toHaveClass(/disabled/);

  await page.locator(".move-down").first().click();
  await expect(entryTitles(page)).toHaveText(["Encore", "Opening Song", "Second Song"]);

  await page.locator(".move-up").last().click();
  await expect(entryTitles(page)).toHaveText(["Encore", "Second Song", "Opening Song"]);
  await expect(page.locator(".move-up").first()).toHaveClass(/disabled/);
  await expect(page.locator(".move-down").last()).toHaveClass(/disabled/);
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

test("enqueue from the playlist view queues the playlist", async ({ page }) => {
  await openMix(page);
  await page.click("#enqueue");

  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song", "Encore", "Second Song"]);
});
