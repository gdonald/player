import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(__dirname, "..");

/// Builds the server and, when it is missing, the client once, before the
/// workers start their servers.
export default function globalSetup() {
  const dist = process.env.CLIENT_DIST ?? path.join(ROOT, "crates/client/dist");

  if (!fs.existsSync(path.join(dist, "index.html"))) {
    execFileSync("trunk", ["build", "--dist", dist], { cwd: path.join(ROOT, "crates/client"), stdio: "inherit" });
  }

  execFileSync("cargo", ["build", "-p", "player-server", "--bin", "player"], { cwd: ROOT, stdio: "inherit" });
}
