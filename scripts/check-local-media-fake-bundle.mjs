#!/usr/bin/env node
/**
 * Proves the local-media E2E fake cannot reach a shipped bundle.
 *
 * This lives in its own gate rather than in the Vitest suite because it performs two real Vite
 * builds. Inside the unit suite that would add minutes to the canonical frontend gate for every
 * developer and every CI run, to assert something that only changes when the build configuration
 * does.
 *
 * The check is two-sided on purpose. The negative case is the guarantee; the positive case is what
 * keeps the negative one from passing vacuously if the flag silently stopped working, which would
 * make every fake-driven E2E assertion meaningless at the same time.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const FAKE_TOKENS = [
  "__vanehubLocalMediaFake",
  "createDeterministicFakeLocalMediaService",
  "fixture recognized line one",
  "fixture transcript",
  "fixture-playback",
];

function bundleText(directory) {
  const chunks = [];
  const walk = (dir) => {
    for (const entry of readdirSync(dir)) {
      const target = join(dir, entry);
      if (statSync(target).isDirectory()) walk(target);
      else if (/\.(js|mjs|css|html)$/.test(entry)) chunks.push(readFileSync(target, "utf8"));
    }
  };
  walk(directory);
  return chunks.join("\n");
}

function build(env, label) {
  const outDir = mkdtempSync(join(tmpdir(), "vanehub-fake-bundle-"));
  process.stdout.write(`building ${label}...\n`);
  execFileSync(
    process.execPath,
    [join(repositoryRoot, "node_modules", "vite", "bin", "vite.js"), "build", "--outDir", outDir, "--emptyOutDir"],
    { cwd: repositoryRoot, env: { ...process.env, ...env }, stdio: "pipe" },
  );
  return outDir;
}

const failures = [];
const built = [];

try {
  const productionDir = build({}, "production bundle (no flag)");
  built.push(productionDir);
  const production = bundleText(productionDir);
  for (const token of FAKE_TOKENS) {
    if (production.includes(token)) failures.push(`production bundle contains "${token}"`);
  }

  const fakeDir = build({ VITE_LOCAL_MEDIA_FAKE: "1" }, "E2E bundle (flag set)");
  built.push(fakeDir);
  const fake = bundleText(fakeDir);
  for (const token of ["__vanehubLocalMediaFake", "fixture transcript"]) {
    if (!fake.includes(token)) failures.push(`E2E bundle is missing "${token}"`);
  }
} finally {
  for (const dir of built) rmSync(dir, { recursive: true, force: true });
}

if (failures.length > 0) {
  console.error("Local-media fake bundle boundary violated:");
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}
console.log("Local-media fake is absent from the production bundle and present only under the flag.");
