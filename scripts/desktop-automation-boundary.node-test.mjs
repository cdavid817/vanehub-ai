import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));

test("keeps desktop automation out of normal production commands and capabilities", async () => {
  const packageJson = await readJson("package.json");
  const defaultCapability = await readJson("src-tauri/capabilities/default.json");
  const productionConfig = await readJson("src-tauri/tauri.conf.json");

  for (const name of ["tauri:build", "package", "package:windows:x64", "package:macos:x64", "package:linux:x64"]) {
    assert.doesNotMatch(packageJson.scripts[name], /desktop-e2e|tauri\.desktop-e2e/);
  }
  assert.equal(productionConfig.app.withGlobalTauri, undefined);
  assert.equal(defaultCapability.permissions.some((permission) => permission.startsWith("wdio")), false);
});

test("requires the explicit test feature, config, and frontend build flag", async () => {
  const cargo = await readFile("src-tauri/Cargo.toml", "utf8");
  const runtime = await readFile("src-tauri/src/bootstrap/runtime.rs", "utf8");
  const frontend = await readFile("src/main.tsx", "utf8");
  const testConfig = await readJson("src-tauri/tauri.desktop-e2e.conf.json");
  const testCapability = testConfig.app.security.capabilities.find((capability) => typeof capability === "object");

  assert.match(cargo, /desktop-e2e = \["dep:tauri-plugin-wdio", "dep:tauri-plugin-wdio-webdriver"\]/);
  assert.match(runtime, /#\[cfg\(feature = "desktop-e2e"\)\][\s\S]*tauri_plugin_wdio::init/);
  assert.match(frontend, /VITE_DESKTOP_E2E === "1"[\s\S]*import\("@wdio\/tauri-plugin"\)/);
  assert.match(frontend, /dataset\.vanehubBootstrap = "ready"/);
  assert.equal(testConfig.app.withGlobalTauri, true);
  assert.equal(testConfig.app.security.capabilities[0], "default");
  assert.equal(testCapability.identifier, "desktop-e2e");
  assert.equal(testCapability.permissions.every((permission) => permission.startsWith("wdio")), true);
});

test("the file-backed credential store exists only in a desktop test build", async () => {
  const credentials = await readFile("src-tauri/src/platform/credentials/mod.rs", "utf8");

  // Two backends, each behind its own cfg, so a production build cannot compile the file-backed
  // one and a test build cannot reach the OS store. Without the split the desktop suite wrote
  // connector tokens into the developer's real Credential Manager and outlived the run.
  assert.match(credentials, /#\[cfg\(not\(feature = "desktop-e2e"\)\)\]\s*\nmod backend \{/);
  assert.match(credentials, /#\[cfg\(feature = "desktop-e2e"\)\]\s*\nmod backend \{/);

  const production = credentials.split('#[cfg(feature = "desktop-e2e")]')[0];
  assert.match(production, /keyring::Entry::new/, "the production backend no longer uses the OS store");

  const testOnly = credentials.split('#[cfg(feature = "desktop-e2e")]')[1] ?? "";
  assert.doesNotMatch(testOnly, /keyring::/, "the test backend must not reach the OS store");
  // Inside the run's isolated data directory, which the orchestrator validates and then deletes.
  assert.match(testOnly, /VANEHUB_APP_DATA_DIR/);
});
