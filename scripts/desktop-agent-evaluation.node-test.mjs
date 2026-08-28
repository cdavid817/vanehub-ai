import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { auditAgentEvaluationEvidence } from "./desktop/agent-evaluation-evidence-safety.mjs";
import {
  evaluateAgentEvaluationPrerequisites,
  hasOpenCodeCredentials,
  writeAgentEvaluationPreflight,
} from "./desktop/agent-evaluation-qualification.mjs";

test("classifies OpenCode fixture and live prerequisites without returning command output", () => {
  assert.equal(evaluateAgentEvaluationPrerequisites({ mode: "fixture-opencode" }).status, "READY");
  const installedWithoutAuth = evaluateAgentEvaluationPrerequisites({
    mode: "live-opencode",
    probe: (_command, args) => ({ ok: true, output: args.includes("auth") ? "0 credentials" : "1.18.21" }),
  });
  assert.equal(installedWithoutAuth.status, "BLOCKED");
  assert.deepEqual(installedWithoutAuth.missingPrerequisites, ["opencode-authentication"]);
  assert.equal(Object.hasOwn(installedWithoutAuth, "output"), false);
  assert.equal(hasOpenCodeCredentials("\u001b[0m 0 credentials"), false);
  assert.equal(hasOpenCodeCredentials("Credentials\n- provider account"), true);
});

test("classifies OnePiece credentials by presence without retaining their value", () => {
  const secret = "onepiece-private-sentinel";
  const result = evaluateAgentEvaluationPrerequisites({
    mode: "live-onepiece",
    env: { VANEHUB_ONEPIECE_API_KEY: secret },
    platform: "linux",
  });
  assert.equal(result.status, "READY");
  assert.equal(JSON.stringify(result).includes(secret), false);
});

test("writes truthful blocked preflight evidence", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-agent-evaluation-preflight-"));
  const preflight = evaluateAgentEvaluationPrerequisites({
    mode: "live-onepiece",
    env: {},
    platform: "linux",
  });
  const result = await writeAgentEvaluationPreflight(root, preflight, new Date("2026-08-27T00:00:00Z"));
  assert.equal(result.status, "BLOCKED");
  const summary = await readFile(result.summaryPath, "utf8");
  assert.match(summary, /onepiece-credential-unavailable/);
  assert.doesNotMatch(summary, /PASSED/);
  await rm(root, { recursive: true, force: true });
});

test("evidence audit rejects a retained provider secret without copying it to the report", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-agent-evaluation-evidence-"));
  await mkdir(path.join(root, "logs"));
  const secret = "onepiece-private-sentinel";
  await writeFile(path.join(root, "logs", "native.log"), `unsafe ${secret}\n`);
  const report = await auditAgentEvaluationEvidence(root, { VANEHUB_ONEPIECE_API_KEY: secret });
  assert.equal(report.status, "FAILED");
  assert.equal(report.findings[0].rule, "provider-secret-retained");
  const retained = await readFile(path.join(root, "agent-evaluation-evidence-safety.json"), "utf8");
  assert.equal(retained.includes(secret), false);
  await rm(root, { recursive: true, force: true });
});
