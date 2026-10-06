import { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { logIn, mp3Row, position, queueTitles, resetData, transport } from "./helpers";

test.beforeEach(async ({ request }) => {
  await resetData(request);
});

async function play(page: Page, title: string) {
  const before = (await queueTitles(page)).length;
  await mp3Row(page, title).locator(".play-mp3").click();
  await expect.poll(async () => (await queueTitles(page)).length).toBe(before + 1);
}


/// Keeps each media key handler the app registers, so a test can press the
/// keys, which pages cannot do on their own.
async function captureMediaKeys(page: Page) {
  await page.addInitScript(() => {
    const handlers: Record<string, (() => void) | null> = {};
    (window as unknown as { mediaKeys: typeof handlers }).mediaKeys = handlers;
    const register = navigator.mediaSession.setActionHandler.bind(navigator.mediaSession);
    navigator.mediaSession.setActionHandler = (action, handler) => {
      handlers[action] = handler as (() => void) | null;
      register(action, handler);
    };
  });
}

function pressMediaKey(page: Page, action: string) {
  return page.evaluate((name) => {
    (window as unknown as { mediaKeys: Record<string, () => void> }).mediaKeys[name]();
  }, action);
}

test("media keys pause, play, skip, restart, and stop", async ({ page }) => {
  await captureMediaKeys(page);
  await logIn(page);
  await play(page, "Encore");
  await play(page, "Opening Song");
  await expect.poll(() => transport(page)).toBe("playing");

  await pressMediaKey(page, "pause");
  await expect.poll(() => transport(page)).toBe("paused");

  await pressMediaKey(page, "play");
  await expect.poll(() => transport(page)).toBe("playing");

  await page.locator("#player-seek").fill("30");
  await pressMediaKey(page, "previoustrack");
  await expect.poll(() => position(page)).toBeLessThan(5);

  await pressMediaKey(page, "nexttrack");
  await expect(page.locator("#player-title")).toContainText("Opening Song");

  await pressMediaKey(page, "stop");
  await expect(page.locator("#player-state")).toHaveClass(/bi-stop-fill/);
});

test("media keys with nothing queued do nothing", async ({ page }) => {
  await captureMediaKeys(page);
  await logIn(page);

  for (const action of ["play", "pause", "stop", "previoustrack", "nexttrack"]) {
    await pressMediaKey(page, action);
  }

  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
});

test("the play key starts a stopped queue", async ({ page }) => {
  await captureMediaKeys(page);
  await logIn(page);
  await play(page, "Encore");
  await page.reload();
  await expect.poll(() => queueTitles(page)).toEqual(["Encore"]);
  await expect.poll(() => transport(page)).toBe("stopped");

  await pressMediaKey(page, "play");

  await expect(page.locator("#player-title")).toContainText("Encore");
});

test("the player works in a browser without the Media Session API", async ({ page }) => {
  await page.addInitScript(() => {
    delete (Navigator.prototype as unknown as { mediaSession?: unknown }).mediaSession;
  });
  await logIn(page);

  await play(page, "Encore");

  await expect.poll(() => transport(page)).toBe("playing");
});

test("a browser that cannot start Web Audio says it cannot play", async ({ page }) => {
  await page.addInitScript(() => {
    window.AudioContext = function () {
      throw new Error("Web Audio is unavailable");
    } as unknown as typeof AudioContext;
  });
  await logIn(page);

  await play(page, "Encore");

  await expect(page.locator(".alert")).toContainText("This browser cannot play audio.");
  await expect(page.locator("#player-title")).toHaveText("Nothing playing");
});

test("a song that cannot be decoded says so and stops", async ({ page }) => {
  await page.route(/\/api\/mp3s\/\d+\/play$/, (route) =>
    route.fulfill({ status: 200, contentType: "audio/mpeg", body: "not audio" }),
  );
  await logIn(page);

  await play(page, "Encore");

  await expect(page.locator(".alert")).toContainText("The song could not be played");
  await expect.poll(() => transport(page)).toBe("stopped");
});

test("changing the equalizer while a song plays keeps it playing", async ({ page }) => {
  await logIn(page);
  await play(page, "Encore");
  await expect.poll(() => transport(page)).toBe("playing");
  await page.click("#equalizer-toggle");

  await page.selectOption("#equalizer-preset", "Rock");

  await expect(page.locator("#equalizer-on")).toHaveAttribute("aria-pressed", "true");
  await expect.poll(() => transport(page)).toBe("playing");
});

test("a song played after the playlist was cleared still plays", async ({ page }) => {
  await logIn(page);
  await play(page, "Encore");
  await expect.poll(() => transport(page)).toBe("playing");
  await page.click("#queue-clear");
  await expect.poll(() => queueTitles(page)).toEqual([]);
  await expect.poll(() => transport(page)).toBe("stopped");

  await play(page, "Opening Song");

  await expect.poll(() => transport(page)).toBe("playing");
});

test("the app works when the browser blocks storage", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      get() {
        throw new Error("storage is blocked");
      },
    });
  });
  await logIn(page);
  await play(page, "Encore");

  await page.selectOption("#theme-select", "charcoal");
  await expect(page.locator("link#theme")).toHaveAttribute("href", "/themes/theme-charcoal.css");

  await page.reload();
  await expect(page.locator("link#theme")).toHaveAttribute("href", "/themes/theme-default.css");
});
