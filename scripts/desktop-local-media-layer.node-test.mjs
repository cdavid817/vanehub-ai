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

/**
 * Every Linux job that compiles the workspace needs the ALSA headers.
 *
 * `cpal` pulls `alsa-sys`, whose build script panics in `pkg-config` when `alsa.pc` is absent, so
 * a job missing `libasound2-dev` fails before it compiles anything -- reported as whatever that job
 * was named, not as a missing package. A job added while this branch was unmerged copied the
 * prerequisite list from before `cpal` existed and failed exactly that way, so the list is checked
 * rather than trusted to be copied correctly next time.
 *
 * The selector is "installs apt packages at all", not "runs cargo": `desktop-smoke` compiles the
 * whole workspace on its Linux leg through `npm run test:desktop`, with no `cargo` token anywhere
 * in the block, and a check keyed on the command would have let exactly that job regress. A Linux
 * job that needs no native build installs nothing and is not selected; one that installs packages
 * for some other reason is asked for one more, which costs install time and nothing else.
 */
test("every Linux CI job that installs build prerequisites installs the ALSA headers", async () => {
  const workflow = (await read(".github/workflows/ci.yml")).replaceAll("\r\n", "\n");
  // Comment lines are dropped first: this very requirement is explained in a comment beside one of
  // the package lists, and a check that reads comments passes on the explanation alone.
  const jobs = workflow
    .split(/\n {2}(?=[a-z][\w-]*:\n)/)
    .map((job) => job.split("\n").filter((line) => !/^\s*#/.test(line)).join("\n"));
  const aptJobs = jobs.filter((job) => job.includes("apt-get install"));

  assert.ok(aptJobs.length >= 4, `expected every Linux build job, found ${aptJobs.length}`);
  for (const job of aptJobs) {
    const name = job.match(/^([\w-]+):/)?.[1] ?? job.slice(0, 40);
    assert.ok(job.includes("libasound2-dev"), `${name} compiles the workspace without libasound2-dev`);
  }
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
