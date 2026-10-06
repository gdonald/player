import { Page, Route } from "@playwright/test";
import { expect, test } from "./fixtures";
import {
  currentQueueRow,
  endTrack,
  logIn,
  mp3Id,
  mp3Row,
  position,
  queueTitles,
  resetData,
  songLength,
  transport,
  waitUntilPlaying,
} from "./helpers";

test.beforeEach(async ({ page, request }) => {
  await resetData(request);
  await logIn(page);
});

/// Decodes, in the page, an MP3 of silent MPEG-1 Layer III frames (128 kbps,
/// 44.1 kHz, mono) behind an Info frame whose LAME tag records the encoder
/// delay and padding, and returns the decoded length in samples.
function decodedLength(page: import("@playwright/test").Page, frames: number, delay: number, padding: number) {
  return page.evaluate(
    async ({ frames, delay, padding }) => {
      const frameBytes = 417;
      const header = [0xff, 0xfb, 0x90, 0xc4];
      const bytes = new Uint8Array(frameBytes * (frames + 1));

      for (let frame = 0; frame <= frames; frame += 1) {
        bytes.set(header, frame * frameBytes);
      }

      const text = (offset: number, value: string) =>
        [...value].forEach((character, index) => (bytes[offset + index] = character.charCodeAt(0)));
      const info = 4 + 17;
      text(info, "Info");
      bytes.set([0, 0, 0, 1], info + 4);
      new DataView(bytes.buffer).setUint32(info + 8, frames);
      const lame = info + 12;
      text(lame, "LAME3.92 ");
      bytes[lame + 21] = delay >> 4;
      bytes[lame + 22] = ((delay & 0x0f) << 4) | (padding >> 8);
      bytes[lame + 23] = padding & 0xff;

      const context = new OfflineAudioContext(1, 1, 44100);
      const buffer = await context.decodeAudioData(bytes.buffer);
      return buffer.length;
    },
    { frames, delay, padding },
  );
}

test("the browser trims MP3 encoder delay and padding when decoding", async ({ page }) => {
  expect(await decodedLength(page, 100, 576, 1344)).toBe(100 * 1152 - 576 - 1344);
});

type Start = { when: number; offset: number; now: number; duration: number };

/// Keeps every buffer source start: when it plays, how far in, the context
/// time of the call, and the song's length.
async function recordStarts(page: Page) {
  await page.addInitScript(() => {
    const starts: Start[] = [];
    (window as unknown as { starts: Start[] }).starts = starts;
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (when = 0, offset = 0, ...rest: number[]) {
      starts.push({ when, offset, now: this.context.currentTime, duration: this.buffer!.duration });
      return start.call(this, when, offset, ...rest);
    };
  });
  await page.reload();
  await expect(page.locator(".library-nav")).toBeVisible();
}

function starts(page: Page) {
  return page.evaluate(() => (window as unknown as { starts: Start[] }).starts);
}

/// The context time a start's song ends.
function endOf(start: Start) {
  return start.when - start.offset + start.duration;
}

async function enqueue(page: Page, title: string) {
  const before = (await queueTitles(page)).length;
  await mp3Row(page, title).locator(".play-mp3").click();
  await expect.poll(async () => (await queueTitles(page)).length).toBe(before + 1);
}

/// Holds requests for one song until `release` is called.
async function holdSong(page: Page, title: string) {
  const id = await mp3Id(title);
  let release: () => void = () => {};
  const released = new Promise<void>((resolve) => (release = resolve));
  await page.route(`**/api/mp3s/${id}/play`, async (route: Route) => {
    await released;
    await route.fallback();
  });
  return release;
}

test("the next song is scheduled to start on the last sample of the current one", async ({ page }) => {
  await recordStarts(page);
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await waitUntilPlaying(page);

  await expect.poll(async () => (await starts(page)).length).toBe(2);
  const [current, next] = await starts(page);

  expect(next.offset).toBe(0);
  expect(next.when).toBeCloseTo(endOf(current), 9);
  expect(next.when).toBeGreaterThan(next.now);
});

test("at the end of a song the scheduled next one plays without a new start", async ({ page }) => {
  await recordStarts(page);
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await waitUntilPlaying(page);
  await expect.poll(async () => (await starts(page)).length).toBe(2);

  await endTrack(page);

  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
  const recorded = await starts(page);
  const [seekStart, scheduled] = recorded.slice(-2);
  expect(scheduled.when).toBeCloseTo(endOf(seekStart), 9);
  await expect.poll(() => transport(page)).toBe("playing");
  await expect.poll(async () => (await starts(page)).length).toBe(recorded.length);
});

test("loop one schedules the same song again on its last sample", async ({ page }) => {
  await recordStarts(page);
  await page.locator("#mode-loop-one").click();
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);

  await expect.poll(async () => (await starts(page)).length).toBe(2);
  const [current, again] = await starts(page);

  expect(again.duration).toBe(current.duration);
  expect(again.when).toBeCloseTo(endOf(current), 9);
});

test("pausing drops the scheduled next song and resuming schedules it again", async ({ page }) => {
  await recordStarts(page);
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await waitUntilPlaying(page);
  await expect.poll(async () => (await starts(page)).length).toBe(2);

  await page.click("#player-pause");
  await expect.poll(() => transport(page)).toBe("paused");
  await page.click("#player-pause");

  await expect.poll(async () => (await starts(page)).length).toBe(4);
  const [, , resumed, rescheduled] = await starts(page);
  expect(rescheduled.when).toBeCloseTo(endOf(resumed), 9);
});

test("a song queued while paused is scheduled when playback resumes", async ({ page }) => {
  await recordStarts(page);
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  await page.click("#player-pause");
  await expect.poll(() => transport(page)).toBe("paused");

  const id = await mp3Id("Opening Song");
  const nextLoaded = page.waitForResponse((response) => response.url().endsWith(`/api/mp3s/${id}/play`));
  await enqueue(page, "Opening Song");
  await nextLoaded;
  await page.click("#player-pause");

  await expect.poll(async () => (await starts(page)).length).toBe(3);
  const [, resumed, scheduled] = await starts(page);
  expect(scheduled.when).toBeCloseTo(endOf(resumed), 9);
});

test("a next song still loading at the end plays as soon as it loads", async ({ page }) => {
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  const release = await holdSong(page, "Opening Song");
  await enqueue(page, "Opening Song");

  await endTrack(page);
  await expect.poll(() => queueTitles(page)).toEqual(["Opening Song"]);
  release();

  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await waitUntilPlaying(page);
});

test("changing the play mode while the next song loads follows the new mode", async ({ page }) => {
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  const release = await holdSong(page, "Opening Song");
  await enqueue(page, "Opening Song");

  await page.locator("#mode-loop-one").click();
  release();
  await endTrack(page);

  await expect(currentQueueRow(page)).toContainText("Encore");
  await expect.poll(() => position(page)).toBeLessThan(5);
  await expect.poll(() => queueTitles(page)).toEqual(["Encore", "Opening Song"]);
});

test("pausing a song while it loads keeps it paused when it arrives", async ({ page }) => {
  const release = await holdSong(page, "Encore");
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect(page.locator("#player-pause")).toBeEnabled();

  await page.click("#player-pause");
  release();

  await songLength(page);
  await expect.poll(() => transport(page)).toBe("paused");
  expect(await position(page)).toBe(0);
});

test("resuming a song while it loads plays it when it arrives", async ({ page }) => {
  const release = await holdSong(page, "Encore");
  await mp3Row(page, "Encore").locator(".play-mp3").click();
  await expect(page.locator("#player-pause")).toBeEnabled();

  await page.click("#player-pause");
  await page.click("#player-pause");
  release();

  await waitUntilPlaying(page);
});

test("a song that is replaced while it loads does not play when it arrives", async ({ page }) => {
  const release = await holdSong(page, "Encore");
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  const encoreId = await mp3Id("Encore");
  const encoreLoaded = page.waitForResponse((response) => response.url().endsWith(`/api/mp3s/${encoreId}/play`));

  await page.click("#player-next");
  await waitUntilPlaying(page);
  release();
  await encoreLoaded;
  const before = await position(page);
  await expect.poll(() => position(page)).toBeGreaterThan(before + 0.5);

  await expect(currentQueueRow(page)).toContainText("Opening Song");
  await expect(page.locator("#player-title")).toContainText("Opening Song");
  await expect.poll(() => transport(page)).toBe("playing");
});

test("next plays an already decoded next song without loading it again", async ({ page }) => {
  await recordStarts(page);
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");
  await waitUntilPlaying(page);
  await expect.poll(async () => (await starts(page)).length).toBe(2);
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));

  await page.click("#player-next");

  await expect(page.locator("#player-title")).toContainText("Opening Song");
  await waitUntilPlaying(page);
  expect(requests.filter((url) => url.endsWith("/play"))).toEqual([]);
});

test("a next song that fails to load shows its error when the queue reaches it", async ({ page }) => {
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  const id = await mp3Id("Opening Song");
  await page.route(`**/api/mp3s/${id}/play`, (route) =>
    route.fulfill({ status: 200, contentType: "audio/mpeg", body: "not audio" }),
  );
  await enqueue(page, "Opening Song");

  await endTrack(page);

  await expect(page.locator(".alert")).toContainText("The song could not be played");
  await expect.poll(() => transport(page)).toBe("stopped");
});

test("a song request with an ended session returns to the login form", async ({ page }) => {
  await page.route(/\/api\/mp3s\/\d+\/play$/, (route) => route.fulfill({ status: 401, body: "" }));

  await mp3Row(page, "Encore").locator(".play-mp3").click();

  await expect(page.locator("#username")).toBeVisible();
});

test("stopping a paused song returns it to the start", async ({ page }) => {
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  await page.locator("#player-seek").fill("30");
  await page.click("#player-pause");
  await expect.poll(() => transport(page)).toBe("paused");

  await page.click("#player-stop");

  await expect.poll(() => transport(page)).toBe("stopped");
  expect(await position(page)).toBe(0);
  await page.locator(".breadcrumb").click();
  await page.keyboard.press(" ");
  await waitUntilPlaying(page);
});

test("seeking while paused moves the position and stays paused", async ({ page }) => {
  await enqueue(page, "Encore");
  await waitUntilPlaying(page);
  await page.click("#player-pause");
  await expect.poll(() => transport(page)).toBe("paused");

  await page.locator("#player-seek").fill("20");

  await expect.poll(() => position(page)).toBe(20);
  await expect.poll(() => transport(page)).toBe("paused");
});

test("the next song is not requested until the current one has loaded", async ({ page }) => {
  const release = await holdSong(page, "Encore");
  const nextId = await mp3Id("Opening Song");
  const requested: string[] = [];
  page.on("request", (request) => requested.push(request.url()));
  await enqueue(page, "Encore");
  await enqueue(page, "Opening Song");

  await expect(page.locator("#player-title")).toContainText("Encore");
  expect(requested.filter((url) => url.endsWith(`/api/mp3s/${nextId}/play`))).toEqual([]);

  const nextRequested = page.waitForRequest((request) => request.url().endsWith(`/api/mp3s/${nextId}/play`));
  release();

  await nextRequested;
  await waitUntilPlaying(page);
});
