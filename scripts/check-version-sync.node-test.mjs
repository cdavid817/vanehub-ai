import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { checkVersionSync, resolveReleaseTag } from "./check-version-sync.mjs";

function fixture({
  packageVersion = "0.1.0-preview.1",
  cargoVersion = packageVersion,
  tauriVersion = packageVersion,
} = {}) {
  const root = mkdtempSync(join(tmpdir(), "vanehub-version-sync-"));
  mkdirSync(join(root, "src-tauri"));
  writeFileSync(join(root, "package.json"), JSON.stringify({ version: packageVersion }), "utf8");
  writeFileSync(
    join(root, "src-tauri", "Cargo.toml"),
    `[package]\nname = "vanehub-ai"\nversion = "${cargoVersion}"\n`,
    "utf8",
  );
  writeFileSync(
    join(root, "src-tauri", "tauri.conf.json"),
    JSON.stringify({ version: tauriVersion }),
    "utf8",
  );
  return root;
}

test("an explicit tag argument takes precedence over the environment", () => {
  const env = { GITHUB_REF_TYPE: "tag", GITHUB_REF_NAME: "v9.9.9" };
  assert.equal(resolveReleaseTag(["node", "script", "v0.1.0-preview.1"], env), "v0.1.0-preview.1");
});

test("a branch ref is not resolved as a release tag", () => {
  const env = { GITHUB_REF_TYPE: "branch", GITHUB_REF_NAME: "main" };
  assert.equal(resolveReleaseTag(["node", "script"], env), undefined);
});

test("a tag ref is resolved from the environment", () => {
  const env = { GITHUB_REF_TYPE: "tag", GITHUB_REF_NAME: "v0.1.0-preview.1" };
  assert.equal(resolveReleaseTag(["node", "script"], env), "v0.1.0-preview.1");
});

test("a manual run on a branch validates versions without comparing a tag", () => {
  const root = fixture();
  const tag = resolveReleaseTag(["node", "script"], {
    GITHUB_REF_TYPE: "branch",
    GITHUB_REF_NAME: "main",
  });
  assert.equal(checkVersionSync(root, tag), "0.1.0-preview.1");
});

test("a matching pre-release tag passes without normalizing the identifier", () => {
  const root = fixture();
  assert.equal(checkVersionSync(root, "v0.1.0-preview.1"), "0.1.0-preview.1");
});

test("a tag whose pre-release identifier differs is rejected", () => {
  const root = fixture();
  assert.throws(
    () => checkVersionSync(root, "v0.1.0-preview.2"),
    /Release tag v0\.1\.0-preview\.2 does not match synchronized version v0\.1\.0-preview\.1/,
  );
});

test("a tag that drops the pre-release identifier is rejected", () => {
  const root = fixture();
  assert.throws(() => checkVersionSync(root, "v0.1.0"), /does not match synchronized version/);
});

test("declarations that disagree on the pre-release identifier are rejected", () => {
  const root = fixture({ tauriVersion: "0.1.0" });
  assert.throws(
    () => checkVersionSync(root, "v0.1.0-preview.1"),
    /Project versions are not synchronized[\s\S]*src-tauri\/tauri\.conf\.json: 0\.1\.0/,
  );
});

test("a stable version still validates against its tag", () => {
  const root = fixture({ packageVersion: "0.1.0" });
  assert.equal(checkVersionSync(root, "v0.1.0"), "0.1.0");
});
