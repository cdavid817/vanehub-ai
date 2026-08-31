import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { checkDependencyUpdates } from "./check-dependency-updates.mjs";

function fixture(configuration, files = []) {
  const root = mkdtempSync(join(tmpdir(), "vanehub-dependency-updates-"));
  mkdirSync(join(root, ".github"), { recursive: true });
  writeFileSync(join(root, ".github", "dependabot.yml"), configuration, "utf8");
  for (const relative of files) {
    const path = join(root, relative);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, "", "utf8");
  }
  return root;
}

const cargoAtRoot = `version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
`;

const cargoAtMember = `version: 2
updates:
  - package-ecosystem: cargo
    directory: /src-tauri
    schedule:
      interval: weekly
`;

test("a directory holding the lockfile passes", () => {
  const root = fixture(cargoAtRoot, ["Cargo.lock"]);

  assert.deepEqual(checkDependencyUpdates(root), [{ ecosystem: "cargo", directory: "/" }]);
});

test("a directory holding a manifest but no lockfile is refused", () => {
  // The exact shape that made the cargo updater idle for months: `src-tauri/Cargo.toml` exists, so
  // the directory looks right, and every dependency in it is `workspace = true` with nothing to
  // bump. A check that accepted a manifest here would accept the defect it exists to catch.
  const root = fixture(cargoAtMember, ["src-tauri/Cargo.toml", "Cargo.lock"]);

  assert.throws(
    () => checkDependencyUpdates(root),
    /`cargo` points at `\/src-tauri`, which holds none of Cargo\.lock/,
  );
});

test("the failure names the ecosystem and says what will happen", () => {
  const root = fixture(cargoAtMember, ["Cargo.lock"]);

  // "Will run and produce nothing" rather than "is misconfigured": the reader has to know that the
  // symptom is silence, because silence is what they will otherwise be looking at.
  assert.throws(() => checkDependencyUpdates(root), /run and produce nothing/);
});

test("an ecosystem with no directory is refused", () => {
  const root = fixture(`version: 2
updates:
  - package-ecosystem: npm
    schedule:
      interval: weekly
`);

  assert.throws(() => checkDependencyUpdates(root), /`npm` names no directory/);
});

test("an ecosystem this check does not know about is refused rather than skipped", () => {
  // Refused, not ignored. An unknown ecosystem silently passing is the same failure one level up:
  // the check reports health for something it did not look at.
  const root = fixture(`version: 2
updates:
  - package-ecosystem: gradle
    directory: /
    schedule:
      interval: weekly
`);

  assert.throws(() => checkDependencyUpdates(root), /does not know what its updater reads/);
});

test("a missing configuration is a failure rather than an absence", () => {
  const root = mkdtempSync(join(tmpdir(), "vanehub-dependency-updates-"));

  assert.throws(() => checkDependencyUpdates(root), /no ecosystem is updated at all/);
});

test("the repository's own configuration is usable", () => {
  // The point of the whole check. If this fails, some ecosystem is not being updated right now.
  const entries = checkDependencyUpdates();

  assert.ok(entries.some((entry) => entry.ecosystem === "cargo"));
  assert.ok(entries.some((entry) => entry.ecosystem === "npm"));
});
