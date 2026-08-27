import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { auditFeishuLiveEvidence } from "./desktop/feishu-live-evidence-safety.mjs";

const privateEnvironment = {
  VANEHUB_FEISHU_TEST_TENANT: "tenant-private-sentinel",
  VANEHUB_FEISHU_APP_ID: "app-id-private-sentinel",
  VANEHUB_FEISHU_APP_SECRET: "app-secret-private-sentinel",
  VANEHUB_FEISHU_TEST_CHAT_ID: "chat-private-sentinel",
};

async function evidenceRoot() {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-feishu-live-evidence-"));
  await mkdir(path.join(root, "logs", "native"), { recursive: true });
  await mkdir(path.join(root, "screenshots"), { recursive: true });
  await writeFile(path.join(root, "summary.json"), "{\"status\":\"NOT RUN\"}\n");
  await writeFile(path.join(root, "wdio-result.json"), "{\"passed\":1}\n");
  await writeFile(path.join(root, "logs", "native", "vanehub.log"), "{\"safeCode\":\"ok\"}\n");
  await writeFile(
    path.join(root, "feishu-live-credential-cleanup.json"),
    "{\"status\":\"CLEARED\",\"credentialProfileOwned\":true}\n",
  );
  return root;
}

test("accepts live evidence after the run-owned credential is cleared", async () => {
  const root = await evidenceRoot();
  const report = await auditFeishuLiveEvidence(root, privateEnvironment);
  assert.equal(report.status, "PASSED");
  assert.equal(report.categories.credentialCleanup, 1);
  assert.deepEqual(report.findings, []);
  await rm(root, { recursive: true, force: true });
});

test("rejects retained runtime values without copying them into the audit report", async () => {
  const root = await evidenceRoot();
  await writeFile(
    path.join(root, "logs", "native", "vanehub.log"),
    privateEnvironment.VANEHUB_FEISHU_APP_SECRET,
  );
  const report = await auditFeishuLiveEvidence(root, privateEnvironment);
  assert.equal(report.status, "FAILED");
  assert.equal(report.findings[0].rule, "runtime-private-value");
  const retained = await readFile(path.join(root, "feishu-live-evidence-safety.json"), "utf8");
  assert.equal(retained.includes(privateEnvironment.VANEHUB_FEISHU_APP_SECRET), false);
  await rm(root, { recursive: true, force: true });
});

test("rejects retained operator messages and external identifiers", async () => {
  const root = await evidenceRoot();
  await writeFile(
    path.join(root, "logs", "native", "vanehub.log"),
    "VANEHUB_LIVE_DIRECT_CHECK oc_externalidentifier123456",
  );
  const report = await auditFeishuLiveEvidence(root, privateEnvironment);
  assert.equal(report.status, "FAILED");
  assert.deepEqual(report.findings.map(({ rule }) => rule), [
    "message-content-retained",
    "external-identifier-retained",
  ]);
  await rm(root, { recursive: true, force: true });
});

test("rejects missing credential cleanup evidence", async () => {
  const root = await evidenceRoot();
  await rm(path.join(root, "feishu-live-credential-cleanup.json"));
  const report = await auditFeishuLiveEvidence(root, privateEnvironment);
  assert.equal(report.status, "FAILED");
  assert.equal(report.findings[0].rule, "credential-cleanup-evidence-missing");
  await rm(root, { recursive: true, force: true });
});
