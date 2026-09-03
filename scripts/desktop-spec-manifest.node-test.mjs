import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import {
  AGENT_EVALUATION_SPECS,
  DESKTOP_SPECS,
  DUPLICATE_REPLACED,
  EXTERNAL_PREREQUISITE_VARIABLES,
  EXTERNAL_PROVIDER,
  REQUIRED_FIXTURE,
  agentEvaluationSpecFiles,
  externalSpecFiles,
  replacedSpecs,
  requiredSpecFiles,
} from "../tests/desktop/spec-manifest.mjs";

const specDir = "tests/desktop/specs";
const specFilesOnDisk = async () =>
  (await readdir(specDir)).filter((entry) => entry.endsWith(".e2e.mjs"));

test("every desktop spec on disk is classified exactly once", async () => {
  const onDisk = await specFilesOnDisk();
  const classified = DESKTOP_SPECS.map((entry) => entry.spec);

  const unclassified = onDisk.filter((spec) => !classified.includes(spec));
  assert.deepEqual(unclassified, [], `unclassified desktop specs: ${unclassified.join(", ")}`);

  const duplicates = classified.filter((spec, index) => classified.indexOf(spec) !== index);
  assert.deepEqual(duplicates, [], `specs classified twice: ${duplicates.join(", ")}`);

  for (const entry of DESKTOP_SPECS) {
    assert.ok(
      [REQUIRED_FIXTURE, EXTERNAL_PROVIDER, DUPLICATE_REPLACED].includes(entry.gate),
      `${entry.spec} has no valid gate`,
    );
  }
});

test("the manifest does not name specs that no longer exist", async () => {
  const onDisk = await specFilesOnDisk();
  const live = DESKTOP_SPECS.filter((entry) => entry.gate !== DUPLICATE_REPLACED);
  const stale = live.filter((entry) => !onDisk.includes(entry.spec)).map((entry) => entry.spec);
  // A renamed or deleted spec has to be re-decided, not silently dropped from the gate.
  assert.deepEqual(stale, [], `manifest entries with no spec file: ${stale.join(", ")}`);
});

test("a replaced spec is really gone and names a replacement that exists", async () => {
  const onDisk = await specFilesOnDisk();
  for (const entry of replacedSpecs()) {
    assert.ok(!onDisk.includes(entry.spec), `${entry.spec} is classified as replaced but still exists`);
    assert.ok(entry.replacedBy, `${entry.spec} names no replacement`);
    const replacement = path.join("tests/desktop", entry.replacedBy);
    await assert.doesNotReject(
      readFile(replacement, "utf8"),
      `${entry.spec} names a replacement that does not exist: ${entry.replacedBy}`,
    );
  }
});

test("no required spec depends on a real credential, provider, or vendor network", async () => {
  const offenders = [];
  for (const spec of requiredSpecFiles()) {
    const source = await readFile(path.join(specDir, spec), "utf8");
    for (const variable of EXTERNAL_PREREQUISITE_VARIABLES) {
      // Whatever such a variable gates is either fixture-resolvable and belongs behind the fixture,
      // or genuinely external and belongs in the other suite. It cannot be in the gate.
      if (source.includes(variable)) offenders.push(`${spec} -> ${variable}`);
    }
  }
  assert.deepEqual(offenders, [], `required specs declaring external prerequisites: ${offenders.join(", ")}`);
});

test("every external spec declares its prerequisites and why it blocks without them", () => {
  for (const entry of DESKTOP_SPECS.filter((candidate) => candidate.gate === EXTERNAL_PROVIDER)) {
    assert.ok(entry.prerequisites?.length, `${entry.spec} declares no prerequisites`);
    assert.ok(entry.blockedReason, `${entry.spec} declares no blocked reason`);
  }
});

test("the required gate runs required specs and never an external one", async () => {
  const config = await readFile("tests/desktop/wdio.conf.mjs", "utf8");
  assert.match(config, /requiredSpecFiles\(\)/);

  const external = externalSpecFiles();
  assert.ok(external.length > 0, "no external spec is classified, so the split is not real");
  for (const spec of external) {
    assert.ok(!requiredSpecFiles().includes(spec), `${spec} is in both gates`);
  }

  // Fixture Agents ahead of the inherited PATH is what makes the gate runnable anywhere.
  assert.match(config, /prepareManagedCliFixtures/);
  // The fixture has to be the *only* Agent reachable, not merely the first. A real installation
  // left elsewhere on PATH is a second installation from a second source, and the launch resolver
  // refuses that pairing as PATH shadowing -- reporting the Agent unavailable while both copies sit
  // there working. That made the gate depend on what the developer happened to have installed.
  assert.match(config, /pathWithoutCompetingAgents/);
});

test("the external suite is never part of the required desktop command", async () => {
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  const requiredLayers = orchestrator.match(/const fullSuiteLayers = \[[^\]]+\]/s)?.[0] ?? "";
  assert.ok(requiredLayers, "the required layer list could not be found");
  assert.ok(
    !requiredLayers.includes("externalProviderDesktop"),
    "the external suite is in the required layer list",
  );
  // `all` is the gate; only the explicit `everything` mode adds the external suite.
  assert.match(orchestrator, /mode === "all" \|\| mode === "everything"/);
  assert.match(orchestrator, /if \(mode === "everything"\)/);
});

test("the focused Agent evaluation layer selects one spec for every provider mode", async () => {
  assert.deepEqual(agentEvaluationSpecFiles("fixture-opencode"), ["agent-evaluation.e2e.mjs"]);
  assert.deepEqual(agentEvaluationSpecFiles("live-opencode"), ["agent-evaluation.e2e.mjs"]);
  assert.deepEqual(agentEvaluationSpecFiles("live-onepiece"), ["agent-evaluation.e2e.mjs"]);
  assert.deepEqual(agentEvaluationSpecFiles("unsupported"), []);
  assert.equal(AGENT_EVALUATION_SPECS.length, 1);

  const config = await readFile("tests/desktop/wdio.agent-evaluation.conf.mjs", "utf8");
  assert.match(config, /agentEvaluationSpecFiles\(mode\)/);
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  assert.match(orchestrator, /agent-evaluation-live-opencode/);
  assert.match(orchestrator, /agent-evaluation-live-onepiece/);
});

test("a blocked external run reports BLOCKED rather than PASSED", async () => {
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  assert.match(orchestrator, /Desktop external provider: BLOCKED/);
  assert.match(orchestrator, /writeExternalBlockedEvidence/);
  // A blocked external suite must not fail a pipeline, and must not be counted as a pass either.
  assert.match(orchestrator, /status: "BLOCKED"/);
});
