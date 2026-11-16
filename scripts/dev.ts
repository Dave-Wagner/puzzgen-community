// Bun only orchestrates the Rust API and Trunk; no JavaScript backend is added.
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const localTrunk = resolve(root, ".tools/trunk/trunk.exe");
const trunk = existsSync(localTrunk) ? localTrunk : "trunk";
const env = { ...process.env, CARGO_BUILD_JOBS: "2", PUZZGEN_LISTEN: "127.0.0.1:8080" };
// Local playground preview is stateless. Never accidentally connect .env's VPS DB.
delete env.DATABASE_URL;
delete env.PUZZGEN_API_KEYS;
const children: ReturnType<typeof Bun.spawn>[] = [];
let stopping = false;
function stop() {
  if (stopping) return;
  stopping = true;
  for (const child of children) {
    if (child.exitCode !== null) continue;
    if (process.platform === "win32") {
      Bun.spawnSync(["taskkill", "/PID", String(child.pid), "/T", "/F"], { stdout: "ignore", stderr: "ignore" });
    } else {
      // Trunk/Cargo can have compiler descendants; terminate their process group.
      try { process.kill(-child.pid, "SIGTERM"); } catch { child.kill(); }
    }
  }
}
process.on("SIGINT", () => { stop(); process.exit(130); });
process.on("SIGTERM", () => { stop(); process.exit(143); });
try {
  console.log("Puzzgen playground: http://127.0.0.1:8081/ (Ctrl+C stops both services)");
  console.log("Local stateless preview; database and API keys are disabled for this command.");
  for (const [cmd, cwd] of [
    [["cargo", "run", "--locked", "-p", "puzzgen", "-j", "2"], root],
    [[trunk, "serve", "--locked", "--skip-version-check"], resolve(root, "frontend")],
  ] as [string[], string][]) {
    children.push(Bun.spawn(cmd, { cwd, env, stdin: "inherit", stdout: "inherit", stderr: "inherit", detached: process.platform !== "win32" }));
  }
  const code = await Promise.race(children.map(child => child.exited));
  process.exitCode = code;
} finally {
  stop();
  await Promise.all(children.map(child => child.exited));
}
