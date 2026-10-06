import { expect, test } from "./fixtures";
import { LIBRARY_ORDER, PASSWORD, USERNAME, alert, logIn, mp3Row, mp3Titles, queueTitles, resetData } from "./helpers";

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
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.click("button[type=submit]");

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

test("the login form sits in a window titled LOGIN", async ({ page }) => {
  await page.goto("/");

  await expect(page.locator("#login .winamp-titlebar span")).toHaveText("LOGIN");
  await expect(page.locator("#login #username")).toBeVisible();
});

test("a theme chosen on the login page stays after signing in", async ({ page }) => {
  await page.goto("/");
  await page.selectOption("#theme-select", "steel");
  await expect(page.locator("link#theme")).toHaveAttribute("href", "/themes/theme-steel.css");

  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.click("button[type=submit]");

  await expect(page.locator(".library-nav")).toBeVisible();
  await expect(page.locator("#theme-select")).toHaveValue("steel");
});

test("the logout button is a small icon in the player's top right corner", async ({ page }) => {
  await logIn(page);

  const button = (await page.locator("#logout").boundingBox())!;
  const player = (await page.locator("#player").boundingBox())!;

  expect(button.width).toBeLessThanOrEqual(24);
  expect(player.x + player.width - (button.x + button.width)).toBeLessThanOrEqual(8);
  expect(button.y - player.y).toBeLessThanOrEqual(8);
  await expect(page.locator("#logout i")).toHaveClass(/bi-box-arrow-right/);
});

test("logging out returns to the login form and ends the session", async ({ page }) => {
  await logIn(page);

  await page.click("#logout");

  await expect(page.locator("#username")).toBeVisible();
  await page.reload();
  await expect(page.locator("#username")).toBeVisible();
});

test("signing in after logging out mid-song starts with nothing playing", async ({ page }) => {
  await logIn(page);
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);

  await page.click("#logout");
  await expect(page.locator("#username")).toBeVisible();
  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.click("button[type=submit]");

  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
  await expect(page.locator("#player-audio")).toHaveCount(0);
});

test("a failed logout shows the error and stays signed in", async ({ page }) => {
  await logIn(page);
  await page.route((url) => url.pathname === "/api/sessions/destroy", (route) => route.fulfill({ status: 500, body: "" }));

  await page.click("#logout");

  await expect(alert(page)).toContainText("Request failed (500)");
  await expect(page.locator(".library-nav")).toBeVisible();
});
