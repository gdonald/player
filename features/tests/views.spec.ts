import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { LIBRARY_ORDER, logIn, mp3Titles, resetData } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

function rows(page: Page, table: string) {
  return page.locator(`#${table} tbody tr`);
}

function modeButton(page: Page, mode: string) {
  return page.locator(`#list-${mode}`);
}

test("the list buttons are a small group in the top right of the page", async ({ page }) => {
  const group = (await page.locator(".list-modes").boundingBox())!;
  const content = (await page.locator(".main-content").boundingBox())!;

  await expect(page.locator(".list-modes button")).toHaveText(["Songs", "Albums", "Artists"]);
  await expect(modeButton(page, "songs")).toHaveAttribute("aria-pressed", "true");
  expect(group.height).toBeLessThanOrEqual(24);
  expect(content.x + content.width - (group.x + group.width)).toBeLessThanOrEqual(24);
  expect(group.y - content.y).toBeLessThanOrEqual(24);
});

test("the Albums button lists each album with its artist and song count", async ({ page }) => {
  await modeButton(page, "albums").click();

  await expect(modeButton(page, "albums")).toHaveAttribute("aria-pressed", "true");
  await expect(rows(page, "albums")).toHaveText([
    /Unknown\s*Filename Artist\s*1/,
    /Live\s*Other Band\s*1/,
    /First Album\s*The Testers\s*2/,
  ]);
});

test("the Artists button lists each artist with album and song counts", async ({ page }) => {
  await modeButton(page, "artists").click();

  await expect(rows(page, "artists")).toHaveText([
    /Filename Artist\s*1\s*1/,
    /Other Band\s*1\s*1/,
    /The Testers\s*1\s*2/,
  ]);
});

test("the Songs button returns to the song list", async ({ page }) => {
  await modeButton(page, "artists").click();
  await expect(rows(page, "artists")).toHaveCount(3);

  await modeButton(page, "songs").click();

  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("clicking an album shows its songs", async ({ page }) => {
  await modeButton(page, "albums").click();

  await rows(page, "albums").filter({ hasText: "First Album" }).locator(".show-album").click();

  await expect(modeButton(page, "songs")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#search")).toHaveValue('artist:"The Testers" album:"First Album"');
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);
});

test("clicking an album's artist shows the artist's songs", async ({ page }) => {
  await modeButton(page, "albums").click();

  await rows(page, "albums").filter({ hasText: "Live" }).locator(".show-artist").click();

  await expect(page.locator("#search")).toHaveValue('artist:"Other Band"');
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
});

test("clicking an artist shows the artist's songs", async ({ page }) => {
  await modeButton(page, "artists").click();

  await rows(page, "artists").filter({ hasText: "The Testers" }).locator(".show-artist").click();

  await expect(modeButton(page, "songs")).toHaveAttribute("aria-pressed", "true");
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);
});

test("clicking an artist's album count shows the artist's albums", async ({ page }) => {
  await modeButton(page, "artists").click();

  await rows(page, "artists").filter({ hasText: "The Testers" }).locator(".show-albums").click();

  await expect(modeButton(page, "albums")).toHaveAttribute("aria-pressed", "true");
  await expect(rows(page, "albums")).toHaveText([/First Album\s*The Testers\s*2/]);
});

test("the search narrows the albums and the artists", async ({ page }) => {
  await modeButton(page, "albums").click();
  await page.fill("#search", "testers");
  await expect(rows(page, "albums")).toHaveText([/First Album/]);

  await modeButton(page, "artists").click();

  await expect(page.locator("#search")).toHaveValue("testers");
  await expect(rows(page, "artists")).toHaveText([/The Testers/]);
});

test("a search with no albums or artists says so", async ({ page }) => {
  await page.fill("#search", "nothing like this");
  await expect(page.getByText("No mp3s found.")).toBeVisible();

  await modeButton(page, "albums").click();
  await expect(page.getByText("No albums found.")).toBeVisible();

  await modeButton(page, "artists").click();
  await expect(page.getByText("No artists found.")).toBeVisible();
});

test("the chosen list is kept across a reload", async ({ page }) => {
  await modeButton(page, "albums").click();
  await expect(rows(page, "albums")).toHaveCount(3);

  await page.reload();

  await expect(modeButton(page, "albums")).toHaveAttribute("aria-pressed", "true");
  await expect(rows(page, "albums")).toHaveCount(3);
});

test("failed album and artist loads show the error", async ({ page }) => {
  await page.route(
    (url) => url.pathname === "/api/albums" || url.pathname === "/api/artists",
    (route) => route.fulfill({ status: 500, body: "" }),
  );

  await modeButton(page, "albums").click();
  await expect(page.locator(".alert")).toContainText("Request failed (500)");

  await page.locator(".alert .btn-close").click();
  await modeButton(page, "artists").click();
  await expect(page.locator(".alert")).toContainText("Request failed (500)");
});

test("clicking an album header sorts by it, ascending then descending", async ({ page }) => {
  await modeButton(page, "albums").click();
  await expect(rows(page, "albums")).toHaveCount(3);

  await page.locator("#albums .sort-album").click();
  await expect(rows(page, "albums").locator(".show-album")).toHaveText(["First Album", "Live", "Unknown"]);

  await page.locator("#albums .sort-album").click();
  await expect(rows(page, "albums").locator(".show-album")).toHaveText(["Unknown", "Live", "First Album"]);

  await page.locator("#albums .sort-songs").click();
  await page.locator("#albums .sort-songs").click();
  await expect(rows(page, "albums").locator(".show-album").first()).toHaveText("First Album");

  await page.locator("#albums .sort-artist").click();
  await page.locator("#albums .sort-artist").click();
  await expect(rows(page, "albums").locator(".show-artist")).toHaveText(["The Testers", "Other Band", "Filename Artist"]);
});

test("clicking an artist header sorts by it, ascending then descending", async ({ page }) => {
  await modeButton(page, "artists").click();
  await expect(rows(page, "artists")).toHaveCount(3);

  await page.locator("#artists .sort-artist").click();
  await page.locator("#artists .sort-artist").click();
  await expect(rows(page, "artists").locator(".show-artist")).toHaveText(["The Testers", "Other Band", "Filename Artist"]);

  await page.locator("#artists .sort-songs").click();
  await page.locator("#artists .sort-songs").click();
  await expect(rows(page, "artists").locator(".show-artist").first()).toHaveText("The Testers");

  await page.locator("#artists .sort-albums").click();
  await expect(rows(page, "artists").locator(".show-artist")).toHaveText(["Filename Artist", "Other Band", "The Testers"]);
});

test("an album's Create Playlist button makes a playlist of its songs and opens it", async ({ page }) => {
  await modeButton(page, "albums").click();

  await rows(page, "albums").filter({ hasText: "First Album" }).locator(".create-playlist").click();

  await expect(page.locator(".alert")).toContainText("Playlist created");
  await expect(page.locator("#name")).toHaveValue("The Testers - First Album");
  await expect(page.locator("#playlist-mp3s .play-mp3")).toHaveText(["Opening Song", "Second Song"]);
  await expect(page.locator(".library-button", { hasText: "Playlists" }).locator(".library-count")).toHaveText("2");
});

test("an artist's Create Playlist button makes a playlist named for the artist", async ({ page }) => {
  await modeButton(page, "artists").click();

  await rows(page, "artists").filter({ hasText: "Other Band" }).locator(".create-playlist").click();

  await expect(page.locator("#name")).toHaveValue("Other Band");
  await expect(page.locator("#playlist-mp3s .play-mp3")).toHaveText(["Encore"]);
});

test("creating a playlist whose name is taken adds a random suffix", async ({ page }) => {
  await modeButton(page, "artists").click();
  const button = rows(page, "artists").filter({ hasText: "Other Band" }).locator(".create-playlist");
  await button.click();
  await expect(page.locator("#name")).toHaveValue("Other Band");
  await page.locator(".library-button", { hasText: "MP3s" }).click();
  await modeButton(page, "artists").click();

  await button.click();

  await expect(page.locator(".alert")).toContainText("Playlist created");
  await expect(page.locator("#name")).toHaveValue(/^Other Band [0-9a-f]{4}$/);
  await expect(page.locator("#playlist-mp3s .play-mp3")).toHaveText(["Encore"]);
});

test("a failed playlist creation shows the error", async ({ page }) => {
  await page.route((url) => url.pathname.endsWith("/playlist"), (route) => route.fulfill({ status: 500, body: "" }));
  await modeButton(page, "albums").click();

  await rows(page, "albums").filter({ hasText: "Live" }).locator(".create-playlist").click();

  await expect(page.locator(".alert")).toHaveText(/Request failed \(500\)$/);
});
