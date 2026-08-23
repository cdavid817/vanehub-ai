import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";

const read = (relative) => readFile(relative, "utf8");

/** Names that must exist only in code compiled out of a production build. */
const ACTIVATION_NAMES = [
  "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES",
  "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE",
  "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT",
  "VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE",
];

test("the local-media layer is wired, named, and reachable on its own", async () => {
  const orchestrator = await read("scripts/test-desktop.mjs");
  const { scripts } = JSON.parse(await read("package.json"));
  const config = await read("tests/desktop/wdio.local-media.conf.mjs");

  assert.match(orchestrator, /mode === "local-media"/);
  assert.match(orchestrator, /layer: "desktop-local-media-fixture"/);
  assert.equal(scripts["test:desktop:local-media"], "node scripts/test-desktop.mjs local-media");
  assert.match(config, /specDirectory: "specs-local-media"/);
});

test("the local-media layer stays out of the default suite and out of the smoke glob", async () => {
  const orchestrator = await read("scripts/test-desktop.mjs");
  const fullSuite = orchestrator.match(/const fullSuiteLayers = \[(.*?)\]/s);

  assert.ok(fullSuite, "the full-suite layer list was not found");
  // The ordinary layers run the same artifact with none of the fixture variables set. Reaching
  // these specs from `all` would run them against the production assembly, where there is no
  // microphone and no engine -- and the failure would look like a product defect.
  assert.doesNotMatch(fullSuite[1], /localMediaDesktop/);
  const specs = await readdir("tests/desktop/specs");
  assert.equal(specs.some((name) => name.includes("local-media-fixture")), false);
});

test("the activation names exist only behind the desktop-e2e feature", async () => {
  const bootstrap = await read("src-tauri/src/bootstrap/local_media.rs");
  const scenario = await read("src-tauri/src/contexts/local_media/infrastructure/fixtures/scenario.rs");

  for (const name of ACTIVATION_NAMES) {
    assert.ok(scenario.includes(name), `${name} is not declared in the gated module`);
  }
  // The bootstrap may branch on the activation, but only inside a `cfg`-gated block, and it must
  // never read the variable itself -- a second reader is a second place the gate can be forgotten.
  assert.match(bootstrap, /#\[cfg\(feature = "desktop-e2e"\)\]\s*\n\s*if let Some\(activation\)/);
  for (const name of ACTIVATION_NAMES) {
    assert.equal(bootstrap.includes(name), false, `${name} is read outside the gated module`);
  }
});

test("the fixture layer refuses a bare interpreter name and a missing interpreter", async () => {
  const fixture = await read("tests/desktop/wdio-local-media-fixture.mjs");

  // The launcher requires an absolute file, so a candidate found on PATH is only a way to ask the
  // interpreter where it actually lives.
  assert.match(fixture, /import sys; print\(sys\.executable\)/);
  assert.match(fixture, /path\.isAbsolute\(resolved\) && isFile\(resolved\)/);
  // Silently skipping would make an unrun layer indistinguishable from a passing one.
  assert.match(fixture, /throw new Error\(\s*`BLOCKED:/);
});

test("the scenario writer is atomic and clears its markers", async () => {
  const helper = await read("tests/desktop/helpers/local-media-scenario.mjs");

  // Both readers re-read the document mid-session and treat malformed JSON as a hard configuration
  // error, so a torn write would abort the application rather than fail a test legibly.
  assert.match(helper, /fsyncSync\(handle\)/);
  assert.match(helper, /renameSync\(temporary, scenarioFile\)/);
  assert.match(helper, /for \(const name of \["crashed", "hang-started", "hang-completed"\]\)/);
});

test("the local-media specs wait on observable state rather than on the clock", async () => {
  const spec = await read("tests/desktop/specs-local-media/domain-local-media-fixture.e2e.mjs");

  assert.doesNotMatch(spec, /browser\.pause\(/);
  assert.doesNotMatch(spec, /waitForTimeout\(/);
  // The single deliberate wait outlives a scripted hang so the absence of its completion marker
  // proves a kill; anything else has to poll.
  assert.equal((spec.match(/await sleep\(/g) ?? []).length, 1);
  assert.ok(spec.includes("browser.waitUntil"), "the spec must poll for observable state");
});
