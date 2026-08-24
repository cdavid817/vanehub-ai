// Reports every migration version number claimed on any ref in this repository.
//
// Migration numbers are dense and global, and parallel branches each pick "the next one" from the
// state they can see. A number chosen from the local branch alone collides the moment two branches
// merge, and the symptom is a startup crash on a database that already ran the other branch's
// migration under the same version. So the choice is made against every ref, not against HEAD.
import { execFileSync } from "node:child_process";

const MIGRATIONS = "src-tauri/src/platform/database/migrations/mod.rs";

function git(args) {
  return execFileSync("git", args, { encoding: "utf8", maxBuffer: 1024 * 1024 * 64 });
}

const refs = git(["for-each-ref", "--format=%(refname)"]).split("\n").filter(Boolean);
const claimed = new Map();
for (const ref of refs) {
  let source;
  try {
    source = git(["show", `${ref}:${MIGRATIONS}`]);
  } catch {
    continue;
  }
  for (const [, version, name] of source.matchAll(/\((\d+),\s*"([^"]+)"\)/g)) {
    const key = Number(version);
    if (!claimed.has(key)) claimed.set(key, new Set());
    claimed.get(key).add(name);
  }
}

const versions = [...claimed.keys()].sort((a, b) => a - b);
const highest = versions.at(-1) ?? 0;
for (const version of versions.slice(-12)) {
  process.stdout.write(`${version}: ${[...claimed.get(version)].join(", ")}\n`);
}
const conflicts = versions.filter((version) => claimed.get(version).size > 1);
if (conflicts.length > 0) {
  process.stdout.write(`\nversions claimed by more than one migration: ${conflicts.join(", ")}\n`);
}
process.stdout.write(`\nhighest claimed on any ref: ${highest}\nnext free: ${highest + 1}\n`);
