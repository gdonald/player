import { Page, Route } from "@playwright/test";
import { expect, test } from "./fixtures";
import { alert, createPlaylist, logIn, menu, mp3Row, queueTitles, resetData, sql } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

/// Answers matching API requests with a status and no body, so the app sees a
/// failed request.
async function failRequests(page: Page, method: string, pathname: RegExp, status = 500) {
  await page.route(
    (url) => pathname.test(url.pathname),
    (route: Route) =>
      route.request().method() === method ? route.fulfill({ status, body: "" }) : route.fallback(),
  );
}

const loginForm = (page: Page) => page.locator("#username");

test("a failed count load shows the error", async ({ page }) => {
  await failRequests(page, "GET", /^\/api\/counts$/);

  await logIn(page);

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed song list load shows the error", async ({ page }) => {
  await failRequests(page, "GET", /^\/api\/mp3s$/);

  await logIn(page);

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed playlist list load on the MP3s page shows the error", async ({ page }) => {
  await failRequests(page, "GET", /^\/api\/playlists$/);

  await logIn(page);

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed queue load shows the error", async ({ page }) => {
  await failRequests(page, "GET", /^\/api\/queued_mp3s$/);

  await logIn(page);

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a dropped connection shows the error", async ({ page }) => {
  await page.route((url) => url.pathname === "/api/counts", (route) => route.abort());

  await logIn(page);

  await expect(alert(page)).toBeVisible();
});

test("a response that is not JSON shows the error", async ({ page }) => {
  await page.route(
    (url) => url.pathname === "/api/counts",
    (route) => route.fulfill({ status: 200, contentType: "application/json", body: "not json" }),
  );

  await logIn(page);

  await expect(alert(page)).toBeVisible();
});

test("a failed enqueue shows the error and plays nothing", async ({ page }) => {
  await failRequests(page, "POST", /^\/api\/queued_mp3s$/);
  await logIn(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect(alert(page)).toContainText("Request failed (500)");
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
});

test("a rejected enqueue shows the server's messages", async ({ page }) => {
  await page.route(
    (url) => url.pathname === "/api/queued_mp3s",
    (route) =>
      route.request().method() === "POST"
        ? route.fulfill({ status: 422, contentType: "application/json", body: '["Mp3 must exist"]' })
        : route.fallback(),
  );
  await logIn(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect(alert(page)).toContainText("Mp3 must exist");
});

test("adding a song from its row menu reports a failed request", async ({ page }) => {
  await failRequests(page, "PUT", /^\/api\/playlists\/\d+$/);
  await logIn(page);

  const row = mp3Row(page, "Encore");
  await row.getByRole("button", { name: "Add to Playlist" }).click();
  await row.locator(".dropdown-item", { hasText: "Recently Played" }).click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed song load on the edit page shows the error", async ({ page }) => {
  await failRequests(page, "GET", /^\/api\/mp3s\/\d+$/);
  await logIn(page);

  await mp3Row(page, "Encore").locator(".edit-mp3").click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("saving a song reports a failed request without field errors", async ({ page }) => {
  await failRequests(page, "PUT", /^\/api\/mp3s\/\d+$/);
  await logIn(page);
  await mp3Row(page, "Encore").locator(".edit-mp3").click();
  await expect(page.locator("#title")).toHaveValue("Encore");

  await page.click("#save");

  await expect(alert(page)).toContainText("Request failed (500)");
  await expect(page.locator(".error").first()).toHaveText("");
});

test("a failed playlist list load on the Playlists page shows the error", async ({ page }) => {
  await logIn(page);
  await failRequests(page, "GET", /^\/api\/playlists$/);

  await menu(page, "Playlists").click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed playlist delete shows the error", async ({ page, request }) => {
  const playlistId = await createPlaylist(request, "Mix", ["Encore"]);
  await failRequests(page, "DELETE", /^\/api\/playlists\/\d+$/);
  await logIn(page);
  await menu(page, "Playlists").click();
  page.once("dialog", (dialog) => dialog.accept());

  await page.locator(`#playlist-${playlistId} .delete-playlist`).click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed playlist load on its edit page shows the error", async ({ page, request }) => {
  const playlistId = await createPlaylist(request, "Mix", ["Encore"]);
  await failRequests(page, "GET", /^\/api\/playlists\/\d+$/);
  await logIn(page);
  await menu(page, "Playlists").click();

  await page.locator(`#playlist-${playlistId} .edit-playlist`).click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed playlist entry load shows the error", async ({ page, request }) => {
  const playlistId = await createPlaylist(request, "Mix", ["Encore"]);
  await failRequests(page, "GET", /^\/api\/playlists\/\d+\/playlist_mp3s$/);
  await logIn(page);
  await menu(page, "Playlists").click();

  await page.locator(`#playlist-${playlistId} .edit-playlist`).click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed source list load shows the error", async ({ page }) => {
  await logIn(page);
  await failRequests(page, "GET", /^\/api\/sources$/);

  await menu(page, "Sources").click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed source load on its edit page shows the error", async ({ page }) => {
  await logIn(page);
  await failRequests(page, "GET", /^\/api\/sources\/\d+$/);
  await menu(page, "Sources").click();

  await page.locator(".edit-source").first().click();

  await expect(alert(page)).toContainText("Request failed (500)");
});

test("a failed scan says so", async ({ page }) => {
  await logIn(page);
  await failRequests(page, "GET", /^\/api\/sources\/\d+\/scan$/);
  await menu(page, "Sources").click();

  await page.locator(".scan-source").first().click();

  await expect(alert(page)).toContainText("Source scanning failed");
});

test("a scan with an ended session returns to the login form", async ({ page }) => {
  await logIn(page);
  await failRequests(page, "GET", /^\/api\/sources\/\d+\/scan$/, 401);
  await menu(page, "Sources").click();

  await page.locator(".scan-source").first().click();

  await expect(loginForm(page)).toBeVisible();
});

test("an ended session returns to the login form while a song plays", async ({ page }) => {
  await logIn(page);
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);

  await sql("DELETE FROM tower_sessions.session");
  await menu(page, "Playlists").click();

  await expect(loginForm(page)).toBeVisible();
  await expect(page.locator("#player")).toHaveCount(0);
});

test("recording a played song with an ended session returns to the login form", async ({ page }) => {
  await failRequests(page, "POST", /^\/api\/mp3s\/\d+\/played$/, 401);
  await logIn(page);

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect(loginForm(page)).toBeVisible();
});

test("a failed source removal shows the error and keeps the source", async ({ page }) => {
  await failRequests(page, "DELETE", /^\/api\/sources\/\d+$/);
  await logIn(page);
  await menu(page, "Sources").click();

  await page.locator("#source-1 .remove-source").click();
  await page.click("#confirm-remove-source");

  await expect(alert(page)).toContainText("Request failed (500)");
  await expect(page.locator("#source-1")).toBeVisible();
});
