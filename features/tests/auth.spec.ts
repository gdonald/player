import { expect, test } from "./fixtures";
import { LIBRARY_ORDER, PASSWORD, USERNAME, logIn, mp3Titles, resetData } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

test("the login form shows before signing in", async ({ page }) => {
  await page.goto("/");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator("#password")).toBeVisible();
});

test("the login fields span most of a phone screen", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");

  for (const field of ["#username", "#password"]) {
    const box = (await page.locator(field).boundingBox())!;
    expect(box.width).toBeGreaterThanOrEqual(300);
  }
});

test("the login fields keep a narrow column on a desktop screen", async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto("/");

  const box = (await page.locator("#username").boundingBox())!;
  expect(box.width).toBeLessThanOrEqual(400);
});

test("a wrong password keeps the login form", async ({ page }) => {
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", "wrong");
  await page.click("button[type=submit]");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator(".library-nav")).toHaveCount(0);
});

test("the right password shows the library", async ({ page }) => {
  await logIn(page);

  await expect.poll(() => mp3Titles(page)).toEqual(LIBRARY_ORDER);
});

test("a reload keeps the session", async ({ page }) => {
  await logIn(page);
  await page.reload();

  await expect(page.locator(".library-nav")).toBeVisible();
  await expect(page.locator("#username")).toHaveCount(0);
});

test("the menu shows a count for each section", async ({ page }) => {
  await logIn(page);

  await expect(page.locator(".library-count")).toHaveText(["4", "1", "1"]);
});

test("the password can be submitted with the enter key", async ({ page }) => {
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.press("#password", "Enter");

  await expect(page.locator(".library-nav")).toBeVisible();
});

test("submitting the login form without a username stays on the form", async ({ page }) => {
  await page.goto("/");
  await page.fill("#password", PASSWORD);

  await page.click("button[type=submit]");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator(".library-nav")).toHaveCount(0);
});

test("submitting the login form without a password stays on the form", async ({ page }) => {
  await page.goto("/");
  await page.fill("#username", USERNAME);

  await page.click("button[type=submit]");

  await expect(page.locator("#username")).toBeVisible();
  await expect(page.locator(".library-nav")).toHaveCount(0);
});
