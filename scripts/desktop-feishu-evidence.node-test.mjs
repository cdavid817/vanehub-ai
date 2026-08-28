import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { auditFeishuEvidence } from "./desktop/feishu-evidence-safety.mjs";

async function createEvidenceRoot() {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-feishu-evidence-"));
  await mkdir(path.join(root, "logs", "native"), { recursive: true });
  await mkdir(path.join(root, "screenshots"), { recursive: true });
  await writeFile(path.join(root, "wdio-result.json"), "{\"exitCode\":0}\n");
  await writeFile(path.join(root, "summary.json"), "{\"status\":\"PASSED\"}\n");
  await writeFile(path.join(root, "feishu-fixture-ledger.json"), "[{\"sequence\":1,\"status\":\"delivered\"}]\n");
  await writeFile(path.join(root, "logs", "wdio.log"), "desktop test passed\n");
  await writeFile(path.join(root, "logs", "native", "vanehub.log"), "{\"safeCode\":\"ok\"}\n");
  return root;
}

test("accepts retained Feishu evidence containing safe metadata only", async () => {
  const root = await createEvidenceRoot();
  const report = await auditFeishuEvidence(root);
  assert.equal(report.status, "PASSED");
  assert.equal(report.categories.fixtureLedger, 1);
  assert.equal(report.categories.resultJson, 2);
  assert.deepEqual(report.findings, []);
  await rm(root, { recursive: true, force: true });
});

test("rejects content, identity, raw protocol, credential, and screenshot evidence safely", async () => {
  const root = await createEvidenceRoot();
  await writeFile(path.join(root, "logs", "wdio.log"), "fixture single agent event\n");
  await writeFile(
    path.join(root, "logs", "native", "vanehub.log"),
    "event_type=im.message.receive_v1 app_secret=hidden-value-123\n",
  );
  await writeFile(path.join(root, "screenshots", "failure.png"), "not-an-image");
  const report = await auditFeishuEvidence(root);
  assert.equal(report.status, "FAILED");
  assert.deepEqual(new Set(report.findings.map(({ rule }) => rule)), new Set([
    "private-fixture-value",
    "raw-feishu-protocol",
    "unredacted-credential",
    "screenshot-content-not-retained",
  ]));
  const retainedReport = await readFile(path.join(root, "feishu-evidence-safety.json"), "utf8");
  assert.equal(retainedReport.includes("fixture single agent event"), false);
  assert.equal(retainedReport.includes("hidden-value-123"), false);
  await rm(root, { recursive: true, force: true });
});
