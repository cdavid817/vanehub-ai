import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

/**
 * Byte-exact snapshot and restore for a generated-artifact directory.
 *
 * A `desktop-e2e` build regenerates `src-tauri/gen/schemas` with the WDIO plugin's ACL entries,
 * which a normal build does not produce. Leaving them behind makes the documentation job's
 * read-only check fail, and the developer who ran a desktop test is rarely the one who finds out.
 *
 * Restoring with `git restore` would be wrong: the directory may already hold edits the user made
 * before running anything, and a Git-based reset would silently destroy them. This snapshots the
 * exact bytes that were present -- modified or not -- and puts those back.
 */
function collect(directory) {
  const files = new Map();
  if (!existsSync(directory)) return files;
  const walk = (current) => {
    for (const entry of readdirSync(current)) {
      const target = path.join(current, entry);
      if (statSync(target).isDirectory()) walk(target);
      else files.set(path.relative(directory, target), readFileSync(target));
    }
  };
  walk(directory);
  return files;
}

function digest(files) {
  const hash = createHash("sha256");
  for (const key of [...files.keys()].sort()) {
    hash.update(key);
    hash.update(files.get(key));
  }
  return hash.digest("hex");
}

export function snapshotDirectory(directory) {
  const files = collect(directory);
  return { directory, files, hash: digest(files) };
}

/**
 * Put the snapshot back and prove it.
 *
 * Files created during the build are removed; files that existed before are rewritten with their
 * original bytes. Anything the caller added *outside* the snapshot but that existed beforehand is
 * untouched, because the snapshot is the authority on what "beforehand" means.
 */
export function restoreDirectory(snapshot) {
  const { directory, files } = snapshot;
  const current = collect(directory);

  const createdDirectories = new Set();
  for (const relative of current.keys()) {
    if (files.has(relative)) continue;
    rmSync(path.join(directory, relative), { force: true });
    // Remember the directories that only existed to hold build output, so the tree shape is
    // restored too rather than left with empty husks.
    let parent = path.dirname(relative);
    while (parent && parent !== ".") {
      createdDirectories.add(parent);
      parent = path.dirname(parent);
    }
  }
  for (const [relative, bytes] of files) {
    const target = path.join(directory, relative);
    const existing = current.get(relative);
    if (existing && existing.equals(bytes)) continue;
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, bytes);
  }

  // Deepest first, and only if empty: a directory that still holds a pre-existing file stays.
  for (const relative of [...createdDirectories].sort((a, b) => b.length - a.length)) {
    const target = path.join(directory, relative);
    if (existsSync(target) && readdirSync(target).length === 0) {
      rmdirSync(target);
    }
  }

  const after = snapshotDirectory(directory);
  if (after.hash !== snapshot.hash) {
    throw new Error(
      `generated schemas could not be restored to their pre-build bytes (${directory})`,
    );
  }
  return after.hash;
}

/** Run `work` with the directory snapshotted, restoring it however `work` ends. */
export async function withDirectoryRestored(directory, work) {
  const snapshot = snapshotDirectory(directory);
  try {
    return await work();
  } finally {
    restoreDirectory(snapshot);
  }
}
