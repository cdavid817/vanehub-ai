import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  parseHostTriple,
  resolvePreparationOptions,
  sidecarPaths,
} from "./prepare-permission-hook-sidecar.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

test("parses the host target and permits an explicit package target", () => {
  const version = "rustc 1.89.0\nhost: x86_64-pc-windows-msvc\n";
  assert.equal(parseHostTriple(version), "x86_64-pc-windows-msvc");
  assert.deepEqual(resolvePreparationOptions([], version), {
    profile: "debug",
    release: false,
    target: "x86_64-pc-windows-msvc",
  });
  assert.deepEqual(
    resolvePreparationOptions(["--release", "--target=aarch64-apple-darwin"], version),
    { profile: "release", release: true, target: "aarch64-apple-darwin" },
  );
});

test("uses Tauri target-qualified staging names for Unix and Windows", () => {
  const windows = sidecarPaths("C:/repo", "x86_64-pc-windows-msvc", "release");
  assert.match(windows.source.replaceAll("\\", "/"), /target\/x86_64-pc-windows-msvc\/release\/vanehub-permission-hook\.exe$/);
  assert.match(windows.staged.replaceAll("\\", "/"), /binaries\/vanehub-permission-hook-x86_64-pc-windows-msvc\.exe$/);

  const macos = sidecarPaths("/repo", "aarch64-apple-darwin", "release");
  assert.match(
    macos.staged.replaceAll("\\", "/"),
    /binaries\/vanehub-permission-hook-aarch64-apple-darwin$/,
  );
});

test("Tauri and npm package entry points cannot bypass sidecar preparation", () => {
  const packageJson = JSON.parse(readFileSync(resolve(repositoryRoot, "package.json"), "utf8"));
  const sidecarConfig = JSON.parse(
    readFileSync(resolve(repositoryRoot, "src-tauri", "tauri.sidecar.conf.json"), "utf8"),
  );
  assert.deepEqual(sidecarConfig.bundle.externalBin, ["binaries/vanehub-permission-hook"]);
  assert.equal(packageJson.scripts.tauri, undefined);

  const expectedTargets = {
    "package:windows:x64": "x86_64-pc-windows-msvc",
    "package:windows:arm64": "aarch64-pc-windows-msvc",
    "package:macos:x64": "x86_64-apple-darwin",
    "package:macos:arm64": "aarch64-apple-darwin",
    "package:linux:x64": "x86_64-unknown-linux-gnu",
    "package:linux:arm64": "aarch64-unknown-linux-gnu",
  };
  for (const [name, target] of Object.entries(expectedTargets)) {
    assert.match(packageJson.scripts[name], new RegExp(`sidecar:prepare -- --release --target=${target}`));
    assert.match(packageJson.scripts[name], /--config src-tauri\/tauri\.sidecar\.conf\.json/);
  }
  assert.match(packageJson.scripts["tauri:dev"], /sidecar:prepare/);
  assert.match(packageJson.scripts["tauri:build"], /sidecar:prepare -- --release/);
});
