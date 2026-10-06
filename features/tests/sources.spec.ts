import { expect, test } from "./fixtures";
import { LIBRARY, alert, logIn, menu, resetData, sql } from "./helpers";

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
