import { expect, test } from "@playwright/test";
import { LIBRARY_ORDER, PASSWORD, USERNAME, logIn, mp3Titles, resetData } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

test("the login form shows before signing in", async ({ page }) => {
  await page.goto("/");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator("#password")).toBeVisible();
});

test("a wrong password keeps the login form", async ({ page }) => {
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", "wrong");
  await page.click("button[type=submit]");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator(".list-group")).toHaveCount(0);
});

test("the right password shows the library", async ({ page }) => {
  await logIn(page);

  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("a reload keeps the session", async ({ page }) => {
  await logIn(page);
  await page.reload();

  await expect(page.locator(".list-group")).toBeVisible();
  await expect(page.locator("#username")).toHaveCount(0);
});

test("the menu shows a count for each section", async ({ page }) => {
  await logIn(page);

  await expect(page.locator(".list-group-item .badge")).toHaveText(["4", "1", "1"]);
});

test("the password can be submitted with the enter key", async ({ page }) => {
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.press("#password", "Enter");

  await expect(page.locator(".list-group")).toBeVisible();
});
