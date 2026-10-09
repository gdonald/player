import { expect, test } from "./fixtures";
import { Page } from "@playwright/test";
import {
  LIBRARY,
  alert,
  logIn,
  menu,
  mp3Row,
  queueTitles,
  resetData,
  serveTone,
  sql,
  transport,
  waitUntilPlaying,
} from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
  await menu(page, "Sources").click();
});

test("sources list their path and mp3 count", async ({ page }) => {
  await expect(page.locator("#source-1 td")).toHaveText([LIBRARY, "4", /Edit/]);
});

test("a source is added from the form", async ({ page }) => {
  await page.fill("#new-source-path", "/zz/music");
  await page.click("#add-source");

  await expect(alert(page)).toContainText("Source created");
  await expect(page.locator("#new-source-path")).toHaveValue("");
  await expect(page.locator("#sources tbody tr td:first-child")).toHaveText([LIBRARY, "/zz/music"]);
});

test("adding a blank or taken path shows an error", async ({ page }) => {
  await page.click("#add-source");
  await expect(page.locator("#new-source .error")).toHaveText("can't be blank");

  await page.fill("#new-source-path", LIBRARY);
  await page.press("#new-source-path", "Enter");
  await expect(page.locator("#new-source .error")).toHaveText("has already been taken");
  await expect(alert(page)).toContainText("Failed to create source");
});

test("the form shows when there are no sources", async ({ page }) => {
  await sql("DELETE FROM sources");
  await menu(page, "Playlists").click();
  await menu(page, "Sources").click();

  await expect(page.getByText("No sources found.")).toBeVisible();
  await expect(page.locator("#new-source-path")).toBeVisible();
});

test("a source path is changed", async ({ page }) => {
  await page.locator("#source-1 .edit-source").click();
  await expect(page.locator("#path")).toHaveValue(LIBRARY);

  await page.fill("#path", "/somewhere/else");
  await page.click("#save");

  await expect(alert(page)).toContainText("Source updated");
  await expect(page.locator(".breadcrumb")).toContainText("/somewhere/else");
});

test("a taken path shows an error", async ({ page }) => {
  await sql("INSERT INTO sources (path) VALUES ('/taken')");
  await page.locator("#source-1 .edit-source").click();
  await page.fill("#path", "/taken");
  await page.click("#save");

  await expect(page.locator(".error")).toHaveText("has already been taken");
});

test("go back returns to the list", async ({ page }) => {
  await page.locator("#source-1 .edit-source").click();
  await page.click("#go-back");

  await expect(page.locator("#source-1")).toBeVisible();
});

test("scanning a source adds its songs", async ({ page }) => {
  await sql("DELETE FROM mp3s");
  await menu(page, "Playlists").click();
  await menu(page, "Sources").click();
  await expect(page.locator("#source-1 td").nth(1)).toHaveText("0");

  await page.locator("#source-1 .scan-source").click();
  await expect(alert(page)).toContainText("Source scanning has been scheduled");

  await expect
    .poll(async () => {
      await menu(page, "Playlists").click();
      await menu(page, "Sources").click();
      return page.locator("#source-1 td").nth(1).textContent();
    })
    .toBe("4");
});

test("adding a source raises the Sources count", async ({ page }) => {
  const sourcesCount = menu(page, "Sources").locator(".library-count");
  await expect(sourcesCount).toHaveText("1");

  await page.fill("#new-source-path", "/zz/music");
  await page.click("#add-source");

  await expect(alert(page)).toContainText("Source created");
  await expect(sourcesCount).toHaveText("2");
});

const removeModal = (page: Page) => page.locator("#remove-source-modal");

async function playFromLibrary(page: Page, title: string) {
  await serveTone(page);
  await menu(page, "MP3s").click();
  await mp3Row(page, title).locator(".play-mp3").click();
  await waitUntilPlaying(page);
  await menu(page, "Sources").click();
}

test("remove asks for confirmation before removing the source", async ({ page }) => {
  await page.locator("#source-1 .remove-source").click();

  await expect(removeModal(page)).toContainText(`Remove ${LIBRARY} from the library?`);
  await expect(removeModal(page)).toContainText("Its 4 MP3s will be removed");
});

test("cancelling the confirmation keeps the source", async ({ page }) => {
  await page.locator("#source-1 .remove-source").click();
  await page.click("#cancel-remove-source");

  await expect(removeModal(page)).toHaveCount(0);
  await expect(page.locator("#source-1")).toBeVisible();
});

test("closing the confirmation keeps the source", async ({ page }) => {
  await page.locator("#source-1 .remove-source").click();
  await removeModal(page).locator(".btn-close").click();

  await expect(removeModal(page)).toHaveCount(0);
  await expect(page.locator("#source-1")).toBeVisible();
});

test("a confirmed removal deletes the source and its songs", async ({ page }) => {
  await page.locator("#source-1 .remove-source").click();
  await page.click("#confirm-remove-source");

  await expect(alert(page)).toContainText("Source removed");
  await expect(page.getByText("No sources found.")).toBeVisible();
  await expect(menu(page, "Sources").locator(".library-count")).toHaveText("0");
  await expect(menu(page, "MP3s").locator(".library-count")).toHaveText("0");
});

test("removing the source of the playing song stops playback and empties the queue", async ({ page }) => {
  await playFromLibrary(page, "Encore");

  await page.locator("#source-1 .remove-source").click();
  await page.click("#confirm-remove-source");

  await expect(alert(page)).toContainText("Source removed");
  await expect.poll(() => queueTitles(page)).toEqual([]);
  await expect.poll(() => transport(page)).toBe("stopped");
});

test("removing another source keeps the playing song", async ({ page }) => {
  await sql("INSERT INTO sources (path) VALUES ('/zz/music')");
  await playFromLibrary(page, "Encore");
  const otherId = (await sql<{ id: string }>("SELECT id FROM sources WHERE path = '/zz/music'"))[0].id;

  await page.locator(`#source-${otherId} .remove-source`).click();
  await page.click("#confirm-remove-source");

  await expect(alert(page)).toContainText("Source removed");
  await expect(page.locator(`#source-${otherId}`)).toHaveCount(0);
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect.poll(() => transport(page)).toBe("playing");
});
