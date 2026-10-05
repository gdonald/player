import { expect, test } from "./fixtures";
import { LIBRARY_ORDER, alert, logIn, mp3Row, mp3Titles, resetData, sql } from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("typing a word filters the list after a pause", async ({ page }) => {
  await page.fill("#search", "encore");

  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
});

test("a quoted phrase matches as one part", async ({ page }) => {
  await page.fill("#search", '"second song"');

  await expect.poll(() => mp3Titles(page)).toEqual(["Second Song"]);
});

test("a search with no matches says so", async ({ page }) => {
  await page.fill("#search", "nothing like this");

  await expect(page.getByText("No mp3s found.")).toBeVisible();
});

test("clicking an artist shows that artist's songs", async ({ page }) => {
  await mp3Row(page, "Opening Song").locator(".search-artist").click();

  await expect(page.locator("#search")).toHaveValue('artist:"The Testers"');
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);
});

test("clicking an album shows that album's songs", async ({ page }) => {
  await mp3Row(page, "Encore").locator(".search-album").click();

  await expect(page.locator("#search")).toHaveValue('album:"Live"');
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
});

test("clear search lists everything again", async ({ page }) => {
  await page.fill("#search", "encore");
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);

  await page.click("#button-addon-search");

  await expect(page.locator("#search")).toHaveValue("");
  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("clicking a column header sorts ascending then descending", async ({ page }) => {
  await page.locator(".sort-title").click();
  await expect
    .poll(() => mp3Titles(page))
    .toEqual(["Encore", "Filename Song", "Opening Song", "Second Song"]);

  await page.locator(".sort-title").click();
  await expect
    .poll(() => mp3Titles(page))
    .toEqual(["Second Song", "Opening Song", "Filename Song", "Encore"]);
});

test("select all checks every row and shows the playlist form", async ({ page }) => {
  await page.check("#select-all");

  await expect(page.locator(".select-mp3:checked")).toHaveCount(4);
  await expect(page.locator("#playlist-select")).toBeVisible();

  await page.uncheck("#select-all");

  await expect(page.locator(".select-mp3:checked")).toHaveCount(0);
  await expect(page.locator("#playlist-select")).toHaveCount(0);
});

test("checking every row by hand checks select all", async ({ page }) => {
  for (const checkbox of await page.locator(".select-mp3").all()) {
    await checkbox.check();
  }

  await expect(page.locator("#select-all")).toBeChecked();
});

test("selected songs are added to an existing playlist", async ({ page }) => {
  await page.check("#select-all");
  await page.selectOption("#playlist-select", { label: "Recently Played" });
  await page.click("#add-selected");

  await expect(alert(page)).toContainText("Playlist updated");
  await expect(page.locator("#name")).toHaveValue("Recently Played");
  await expect(page.locator("#playlist-mp3s tbody tr")).toHaveCount(4);
});

test("a new playlist from an album search takes the album name", async ({ page }) => {
  await mp3Row(page, "Opening Song").locator(".search-album").click();
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);

  await page.check("#select-all");
  await page.selectOption("#playlist-select", "new");

  await expect(page.locator("#new-playlist-name")).toHaveValue("First Album");

  await page.click("#create-playlist");

  await expect(alert(page)).toContainText("Playlist created");
  await expect(page.locator("#name")).toHaveValue("First Album");
  await expect(page.locator("#playlist-mp3s .play-mp3")).toHaveText(["Opening Song", "Second Song"]);
});

test("a taken playlist name shows an error", async ({ page }) => {
  await page.check("#select-all");
  await page.selectOption("#playlist-select", "new");
  await page.fill("#new-playlist-name", "Recently Played");
  await page.click("#create-playlist");

  await expect(page.locator(".playlists .error")).toHaveText("has already been taken");
  await expect(alert(page)).toContainText("Failed to create playlist");
});

test("cancel returns to the playlist picker", async ({ page }) => {
  await page.check("#select-all");
  await page.selectOption("#playlist-select", "new");
  await page.getByRole("button", { name: "Cancel" }).click();

  await expect(page.locator("#playlist-select")).toHaveValue("0");
});

test("a row's playlist menu adds that song", async ({ page }) => {
  const row = mp3Row(page, "Encore");
  await row.getByRole("button", { name: "Add to Playlist" }).click();
  await row.locator(".dropdown-item", { hasText: "Recently Played" }).click();

  await expect(alert(page)).toContainText("Playlist updated");

  const rows = await sql("SELECT 1 FROM playlist_mp3s");
  expect(rows).toHaveLength(1);
});

test("playlist menu items exist only for the open row", async ({ page }) => {
  await expect(page.locator("#mp3s .dropdown-item")).toHaveCount(0);

  await mp3Row(page, "Encore").getByRole("button", { name: "Add to Playlist" }).click();

  await expect(page.locator("#mp3s .dropdown-menu")).toHaveCount(1);
});

test("sorting reorders the existing rows instead of rebuilding them", async ({ page }) => {
  await mp3Row(page, "Encore").evaluate((row) => row.setAttribute("data-marker", "kept"));

  await page.locator(".sort-title").click();
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore", "Filename Song", "Opening Song", "Second Song"]);

  await expect(mp3Row(page, "Encore")).toHaveAttribute("data-marker", "kept");
});

test("a row's playlist menu closes on an outside click", async ({ page }) => {
  const row = mp3Row(page, "Encore");
  await row.getByRole("button", { name: "Add to Playlist" }).click();
  await expect(row.locator(".dropdown-menu")).toBeVisible();

  await page.locator(".breadcrumb").click();

  await expect(row.locator(".dropdown-menu")).toBeHidden();
});

test("editing a song saves its tags", async ({ page }) => {
  await mp3Row(page, "Encore").locator(".edit-mp3").click();
  await expect(page.locator("#title")).toHaveValue("Encore");

  await page.fill("#title", "Encore Live");
  await page.fill("#track", "7");
  await page.click("#save");

  await expect(alert(page)).toContainText("MP3 updated");

  await page.click("#go-back");

  await expect(alert(page)).toHaveCount(0);
  await expect(mp3Row(page, "Encore Live").locator("td").nth(3)).toHaveText("7");
});

test("editing shows each invalid field", async ({ page }) => {
  await mp3Row(page, "Encore").locator(".edit-mp3").click();
  await page.fill("#title", "");
  await page.fill("#track", "seven");
  await page.click("#save");

  await expect(alert(page)).toContainText("Failed to update mp3");
  await expect(page.locator(".error")).toHaveText(["can't be blank", "", "", "is not a number"]);
});

test("the edit breadcrumb returns to the list", async ({ page }) => {
  await mp3Row(page, "Encore").locator(".edit-mp3").click();
  await page.locator(".breadcrumb a").click();

  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("the search is kept across a reload", async ({ page }) => {
  await page.fill("#search", "encore");
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);

  await page.reload();

  await expect(page.locator("#search")).toHaveValue("encore");
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
});

test("clear search forgets the kept search", async ({ page }) => {
  await page.fill("#search", "encore");
  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
  await page.click("#button-addon-search");
  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);

  await page.reload();

  await expect(page.locator("#search")).toHaveValue("");
  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("an artist link search is kept across a reload", async ({ page }) => {
  await mp3Row(page, "Opening Song").locator(".search-artist").click();
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);

  await page.reload();

  await expect(page.locator("#search")).toHaveValue('artist:"The Testers"');
  await expect.poll(() => mp3Titles(page)).toEqual(["Opening Song", "Second Song"]);
});

test("the search button is labeled Clear", async ({ page }) => {
  await expect(page.locator("#button-addon-search")).toHaveText("Clear");
});

test("typing quickly searches once for the whole word", async ({ page }) => {
  await page.locator("#search").pressSequentially("encore", { delay: 20 });

  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
});

test("a row's playlist menu closes when its button is clicked again", async ({ page }) => {
  const button = mp3Row(page, "Encore").getByRole("button", { name: "Add to Playlist" });
  await button.click();
  await expect(mp3Row(page, "Encore").locator(".dropdown-menu")).toBeVisible();

  await button.click();

  await expect(mp3Row(page, "Encore").locator(".dropdown-menu")).toHaveCount(0);
});

test("adding selected songs with no playlist chosen does nothing", async ({ page }) => {
  await page.check("#select-all");

  await page.click("#add-selected");

  await expect(alert(page)).toHaveCount(0);
  expect(await sql("SELECT 1 FROM playlist_mp3s")).toHaveLength(0);
});

test("saving a song with a blank track clears its track", async ({ page }) => {
  await mp3Row(page, "Encore").locator(".edit-mp3").click();
  await expect(page.locator("#title")).toHaveValue("Encore");

  await page.fill("#track", "");
  await page.click("#save");

  await expect(alert(page)).toContainText("MP3 updated");
  const rows = await sql<{ track: number | null }>("SELECT track FROM mp3s WHERE title = 'Encore'");
  expect(rows[0].track).toBeNull();
});

test("pressing Enter in the search box searches without reloading the page", async ({ page }) => {
  await page.fill("#search", "encore");

  await page.locator("#search").press("Enter");

  await expect.poll(() => mp3Titles(page)).toEqual(["Encore"]);
  await expect(page.locator("#search")).toHaveValue("encore");
});
