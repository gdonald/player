import { execFileSync } from "node:child_process";
import path from "node:path";
import { APIRequestContext, Page, expect } from "@playwright/test";
import { Client } from "pg";
import { DATABASE_URL, LIBRARY } from "./worker";

export { LIBRARY };

const ROOT = path.resolve(__dirname, "../..");

/// The login each worker's server seeds (see fixtures.ts).
export const USERNAME = "tester";
export const PASSWORD = "testing-password";

export const LIBRARY_ORDER = ["Filename Song", "Encore", "Opening Song", "Second Song"];

export async function sql<Row = Record<string, unknown>>(query: string, params: unknown[] = []): Promise<Row[]> {
  const client = new Client({ connectionString: DATABASE_URL });
  await client.connect();

  try {
    const result = await client.query(query, params);
    return result.rows as Row[];
  } finally {
    await client.end();
  }
}

export async function apiLogIn(request: APIRequestContext) {
  const response = await request.post("/api/sessions", {
    data: { session: { username: USERNAME, password: PASSWORD } },
  });
  expect(response.ok()).toBeTruthy();
}

/// Rewrites the demo library, empties every table but users, and scans the
/// library again, so each test starts from the same four songs.
export async function resetData(request: APIRequestContext) {
  execFileSync(path.join(ROOT, "target/debug/player"), ["demo-library", LIBRARY], {
    env: { ...process.env, DATABASE_URL },
  });

  await sql(
    "TRUNCATE queued_mp3s, playlist_mp3s, playlists, mp3s, albums, artists, sources RESTART IDENTITY CASCADE",
  );
  await sql("INSERT INTO sources (path) VALUES ($1)", [LIBRARY]);
  await sql("INSERT INTO playlists (name) VALUES ('Recently Played')");

  await apiLogIn(request);
  expect((await request.get("/api/sources/1/scan")).ok()).toBeTruthy();

  await expect
    .poll(async () => (await (await request.get("/api/counts")).json()).mp3s_count)
    .toBe(4);
}

export async function mp3Id(title: string): Promise<number> {
  const rows = await sql<{ id: string }>("SELECT id FROM mp3s WHERE title = $1", [title]);
  return Number(rows[0].id);
}

export async function createPlaylist(request: APIRequestContext, name: string, titles: string[]) {
  const ids = await Promise.all(titles.map(mp3Id));
  const response = await request.post("/api/playlists", {
    data: { playlist: { name, playlist_mp3s_attributes: ids.map((id) => ({ mp3_id: id })) } },
  });
  expect(response.ok()).toBeTruthy();

  return (await response.json()).playlist.id as number;
}

/// Signs in through the API, which shares the session cookie with the page,
/// then opens the app. The login form has its own tests in auth.spec.ts.
export async function logIn(page: Page) {
  await apiLogIn(page.request);
  await page.goto("/");
  await expect(page.locator(".library-nav")).toBeVisible();
}

export function alert(page: Page) {
  return page.locator(".alert");
}

export function menu(page: Page, label: string) {
  return page.locator(".library-button", { hasText: label });
}

export async function mp3Titles(page: Page) {
  return page.locator("#mp3s .play-mp3").allTextContents();
}

export function mp3Row(page: Page, title: string) {
  return page.locator("#mp3s tbody tr", { has: page.locator(".play-mp3", { hasText: title }) });
}

export async function queueTitles(page: Page) {
  return page.locator("#queue .queue-title").allTextContents();
}

export function currentQueueRow(page: Page) {
  return page.locator("#queue tr.table-primary");
}

/// What the player's display shows: "playing", "paused", or "stopped".
export async function transport(page: Page) {
  const classes = (await page.locator("#player-state").getAttribute("class")) ?? "";
  return classes.includes("bi-play-fill") ? "playing" : classes.includes("bi-pause-fill") ? "paused" : "stopped";
}

/// Seconds into the current song, from the seek bar.
export async function position(page: Page) {
  return Number(await page.locator("#player-seek").inputValue());
}

/// The current song's length in seconds, once it has loaded.
export async function songLength(page: Page) {
  await expect.poll(async () => Number(await page.locator("#player-seek").getAttribute("max"))).toBeGreaterThan(0);
  return Number(await page.locator("#player-seek").getAttribute("max"));
}

/// Waits until the current song is loaded and playing.
export async function waitUntilPlaying(page: Page) {
  await songLength(page);
  await expect.poll(() => transport(page)).toBe("playing");
  await expect.poll(() => position(page)).toBeGreaterThan(0);
}

/// Lets the playing song run to its end by seeking to just before it.
export async function endTrack(page: Page) {
  const length = await songLength(page);
  await page.locator("#player-seek").fill((length - 0.3).toFixed(2));
}

/// A 16-bit mono WAV of a 440 Hz tone.
export function toneWav(seconds: number) {
  const sampleRate = 44100;
  const samples = sampleRate * seconds;
  const wav = Buffer.alloc(44 + samples * 2);

  wav.write("RIFF", 0);
  wav.writeUInt32LE(36 + samples * 2, 4);
  wav.write("WAVE", 8);
  wav.write("fmt ", 12);
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20);
  wav.writeUInt16LE(1, 22);
  wav.writeUInt32LE(sampleRate, 24);
  wav.writeUInt32LE(sampleRate * 2, 28);
  wav.writeUInt16LE(2, 32);
  wav.writeUInt16LE(16, 34);
  wav.write("data", 36);
  wav.writeUInt32LE(samples * 2, 40);
  for (let index = 0; index < samples; index += 1) {
    wav.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 440 * index) / sampleRate) * 20000), 44 + index * 2);
  }

  return wav;
}

/// Serves a tone for every song, since the demo library's MP3s are silent.
export async function serveTone(page: Page, seconds = 5) {
  const wav = toneWav(seconds);
  await page.route(/\/api\/mp3s\/\d+\/play$/, (route) =>
    route.fulfill({ status: 200, contentType: "audio/wav", body: wav }),
  );
}
