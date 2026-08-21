import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/package.yml", "utf8").replaceAll("\r\n", "\n");
const stableNotes = readFileSync(".github/STABLE_RELEASE_NOTES.md", "utf8");
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));

test("stable and preview tags select their reviewed release notes", () => {
  assert.ok(
    workflow.includes(
      "args+=(--prerelease --latest=false --notes-file .github/PREVIEW_RELEASE_NOTES.md)",
    ),
  );
  assert.ok(workflow.includes("args+=(--notes-file .github/STABLE_RELEASE_NOTES.md)"));
  assert.ok(workflow.includes('if [[ "${GITHUB_REF_NAME}" == *-* ]]'));
});

test("stable updater-only phase fails closed without the updater signing key", () => {
  assert.ok(workflow.includes('if [[ -z "${TAURI_SIGNING_PRIVATE_KEY}" ]]'));
  assert.ok(workflow.includes('if [[ "${GITHUB_REF_NAME}" != *-* ]]'));
  assert.ok(
    workflow.includes(
      "publishing an explicitly unsigned Windows package for updater-only phase 1",
    ),
  );
  assert.ok(
    workflow.includes(
      "publishing an explicitly unsigned and un-notarized macOS package for updater-only phase 1",
    ),
  );
});

test("the first stable release keeps the rehearsed five-target matrix", () => {
  const matrix = workflow.match(/ {6}matrix:\n {8}include:\n([\s\S]*?) {4}steps:/)?.[1];
  assert.ok(matrix, "Package matrix was not found.");
  const expected = [
    ["windows", "x64", "windows-latest", "x86_64-pc-windows-msvc"],
    ["macos", "arm64", "macos-14", "aarch64-apple-darwin"],
    ["macos", "x64", "macos-15-intel", "x86_64-apple-darwin"],
    ["linux", "x64", "ubuntu-latest", "x86_64-unknown-linux-gnu"],
    ["linux", "arm64", "ubuntu-24.04-arm", "aarch64-unknown-linux-gnu"],
  ];

  assert.equal([...matrix.matchAll(/ {10}- platform:/g)].length, expected.length);
  for (const [platform, arch, runner, rustTarget] of expected) {
    const entry = [
      `          - platform: ${platform}`,
      `            arch: ${arch}`,
      `            runner: ${runner}`,
      `            rust_target: ${rustTarget}`,
    ].join("\n");
    assert.ok(matrix.includes(entry), `Missing release target ${platform}-${arch}.`);
  }
  assert.deepEqual(tauriConfig.bundle.targets, ["nsis", "app", "dmg", "deb", "appimage"]);
});

test("Linux runners install every native AppImage prerequisite", () => {
  assert.match(workflow, /wget \\\n {12}xdg-utils/);
});

test("rehearsal signing exports the private key variable used by the bundler", () => {
  assert.ok(workflow.includes('echo "TAURI_SIGNING_PRIVATE_KEY=${key_path}" >> "${GITHUB_ENV}"'));
  assert.ok(workflow.includes('echo "TAURI_SIGNING_PRIVATE_KEY_PATH=${key_path}" >> "${GITHUB_ENV}"'));
});

test("stable notes distinguish platform signing from Linux integrity evidence", () => {
  for (const heading of ["## Downloads", "## Verify your download", "## Updates", "## Known limitations", "## Reporting problems"]) {
    assert.ok(stableNotes.includes(heading), `Missing stable release section: ${heading}`);
  }
  assert.match(stableNotes, /Windows packages are not Authenticode signed/);
  assert.match(stableNotes, /macOS packages are not Developer ID signed or notarized/);
  assert.match(stableNotes, /Linux packages do not use operating-system code signing/);
});
