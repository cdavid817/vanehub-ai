import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  restoreDirectory,
  snapshotDirectory,
  withDirectoryRestored,
} from "./desktop/schema-snapshot.mjs";

function scratch() {
  return mkdtempSync(path.join(tmpdir(), "vanehub-schema-snapshot-"));
}

function listing(directory) {
  return readdirSync(directory, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => path.relative(directory, path.join(entry.parentPath ?? entry.path, entry.name)))
    .sort();
}

test("restores a directory the build left untouched", () => {
  const directory = scratch();
  writeFileSync(path.join(directory, "desktop-schema.json"), "{}");
  const snapshot = snapshotDirectory(directory);

  restoreDirectory(snapshot);

  assert.equal(readFileSync(path.join(directory, "desktop-schema.json"), "utf8"), "{}");
  rmSync(directory, { recursive: true, force: true });
});

test("preserves a modification the user made before the build", () => {
  const directory = scratch();
  const file = path.join(directory, "desktop-schema.json");
  // The user's own edit is what the snapshot captures. A Git-based restore would replace it with
  // HEAD and destroy work nobody asked to discard.
  writeFileSync(file, '{"user":"edit"}');
  const snapshot = snapshotDirectory(directory);

  writeFileSync(file, '{"build":"output"}');
  restoreDirectory(snapshot);

  assert.equal(readFileSync(file, "utf8"), '{"user":"edit"}');
  rmSync(directory, { recursive: true, force: true });
});

test("rewrites a file the build overwrote", () => {
  const directory = scratch();
  const file = path.join(directory, "windows-schema.json");
  writeFileSync(file, "original");
  const snapshot = snapshotDirectory(directory);

  writeFileSync(file, "regenerated with wdio entries");
  restoreDirectory(snapshot);

  assert.equal(readFileSync(file, "utf8"), "original");
  rmSync(directory, { recursive: true, force: true });
});

test("deletes a file the build created", () => {
  const directory = scratch();
  writeFileSync(path.join(directory, "kept.json"), "kept");
  const snapshot = snapshotDirectory(directory);

  writeFileSync(path.join(directory, "acl-manifests.json"), "new");
  mkdirSync(path.join(directory, "nested"), { recursive: true });
  writeFileSync(path.join(directory, "nested", "deep.json"), "new");
  restoreDirectory(snapshot);

  assert.deepEqual(listing(directory), ["kept.json"]);
  rmSync(directory, { recursive: true, force: true });
});

test("restores after the build throws", async () => {
  const directory = scratch();
  const file = path.join(directory, "desktop-schema.json");
  writeFileSync(file, "original");

  await assert.rejects(
    withDirectoryRestored(directory, async () => {
      writeFileSync(file, "half-written");
      throw new Error("build failed");
    }),
    /build failed/,
  );

  assert.equal(readFileSync(file, "utf8"), "original");
  rmSync(directory, { recursive: true, force: true });
});

test("reports a hash mismatch rather than claiming success", () => {
  const directory = scratch();
  writeFileSync(path.join(directory, "desktop-schema.json"), "original");
  const snapshot = snapshotDirectory(directory);

  // A corrupted snapshot must not restore quietly: the whole point is a provable return to the
  // pre-build bytes.
  snapshot.hash = "0".repeat(64);
  assert.throws(() => restoreDirectory(snapshot), /could not be restored/);
  rmSync(directory, { recursive: true, force: true });
});

test("treats an absent directory as empty rather than failing", () => {
  const directory = path.join(scratch(), "never-created");

  const snapshot = snapshotDirectory(directory);

  assert.equal(snapshot.files.size, 0);
  assert.doesNotThrow(() => restoreDirectory(snapshot));
});
