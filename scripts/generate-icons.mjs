import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

// One vector source supplies the sidebar, browser favicon and native branding.
const root = fileURLToPath(new URL("../", import.meta.url));
const output = resolve(root, "src-tauri/target/branding-icons");
const icons = resolve(root, "src-tauri/icons");
mkdirSync(output, { recursive: true });
const generated = spawnSync(
  process.execPath,
  [
    resolve(root, "node_modules/@tauri-apps/cli/tauri.js"),
    "icon",
    resolve(root, "public/contextdock.svg"),
    "--output",
    output,
  ],
  { cwd: root, stdio: "inherit" },
);
if (generated.error) throw generated.error;
if (generated.status !== 0) process.exit(generated.status ?? 1);
// Keep the existing supported native asset set; mobile outputs stay temporary.
for (const name of readdirSync(icons)) {
  if (/\.(png|ico)$/.test(name)) {
    copyFileSync(resolve(output, name), resolve(icons, name));
  }
}
copyFileSync(resolve(output, "icon.png"), resolve(root, "app-icon.png"));
