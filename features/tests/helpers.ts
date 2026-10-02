import { execFileSync } from "node:child_process";
import path from "node:path";
import { APIRequestContext, Page, expect } from "@playwright/test";
import { Client } from "pg";

const ROOT = path.resolve(__dirname, "../..");
export const LIBRARY = path.resolve(__dirname, "../.library");

export const USERNAME = "gd";
export const PASSWORD = "changeme";

export const LIBRARY_ORDER = ["Filename Song", "Encore", "Opening Song", "Second Song"];

export async function sql<Row = Record<string, unknown>>(query: string, params: unknown[] = []): Promise<Row[]> {
  const client = new Client({ connectionString: process.env.DATABASE_URL });
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
  execFileSync(path.join(ROOT, "target/debug/examples/demo_library"), [LIBRARY]);

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

export async function logIn(page: Page) {
  await page.goto("/");
  await page.fill("#username", USERNAME);
  await page.fill("#password", PASSWORD);
  await page.click("button[type=submit]");
  await expect(page.locator(".list-group")).toBeVisible();
}

export function alert(page: Page) {
  return page.locator(".alert");
}

export function menu(page: Page, label: string) {
  return page.locator(".list-group-item", { hasText: label });
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

export async function endTrack(page: Page) {
  await page.locator("audio").evaluate((audio) => audio.dispatchEvent(new Event("ended")));
}
