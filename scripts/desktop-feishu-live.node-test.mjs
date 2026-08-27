import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  evaluateFeishuLivePrerequisites,
  FEISHU_LIVE_OPT_IN,
  writeFeishuLivePreflight,
} from "./desktop/feishu-live-qualification.mjs";
import { createRunContext, disposeRunContext } from "./desktop/run-context.mjs";

const completeEnvironment = {
  [FEISHU_LIVE_OPT_IN]: "1",
  VANEHUB_FEISHU_TEST_TENANT: "private-tenant-value",
  VANEHUB_FEISHU_APP_ID: "private-app-id",
  VANEHUB_FEISHU_APP_SECRET: "private-app-secret",
  VANEHUB_FEISHU_PERMISSIONS_CONFIRMED: "1",
  VANEHUB_FEISHU_LONG_CONNECTION_CONFIRMED: "1",
  VANEHUB_FEISHU_TEST_CHAT_ID: "private-chat-id",
};

test("live Feishu qualification requires explicit opt-in", () => {
  const result = evaluateFeishuLivePrerequisites({});
  assert.equal(result.status, "NOT RUN");
  assert.equal(result.reason, "explicit-opt-in-required");
  assert.equal(result.optedIn, false);
});

test("live Feishu qualification reports every unavailable prerequisite", () => {
  const result = evaluateFeishuLivePrerequisites({ [FEISHU_LIVE_OPT_IN]: "1" });
  assert.equal(result.status, "BLOCKED");
  assert.deepEqual(result.missingPrerequisites, [
    "dedicated-tenant",
    "app-id",
    "app-secret",
    "bot-message-permissions",
    "long-connection-subscription",
    "direct-test-chat",
  ]);
});

test("live Feishu qualification recognizes a complete preflight without claiming a pass", async () => {
  assert.equal(evaluateFeishuLivePrerequisites(completeEnvironment).status, "READY");
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-feishu-live-"));
  const result = await writeFeishuLivePreflight(root, completeEnvironment, new Date("2026-08-25T00:00:00Z"));
  assert.equal(result.status, "NOT RUN");
  assert.equal(result.reason, "live-qualification-execution-pending");
  const summary = await readFile(result.summaryPath, "utf8");
  for (const value of Object.values(completeEnvironment)) {
    if (value !== "1") assert.equal(summary.includes(value), false, "preflight evidence leaked a value");
  }
  assert.match(summary, /"livePlatform": true/);
  assert.match(summary, /"fixture": false/);
  await rm(root, { recursive: true, force: true });
});

test("live Feishu desktop state and retained evidence use isolated scopes", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-feishu-live-context-"));
  const context = await createRunContext(path.join(root, "repo"), {
    tempRoot: root,
    resultScope: "desktop-live",
  });
  assert.equal(context.dataDir.startsWith(context.runRoot), true);
  assert.match(context.resultDir.replaceAll("\\", "/"), /test-results\/desktop-live\//);
  await disposeRunContext(context);
  await rm(root, { recursive: true, force: true });
});

test("the live Feishu entry point is explicit and excluded from deterministic desktop all", async () => {
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  const { scripts } = JSON.parse(await readFile("package.json", "utf8"));
  const config = await readFile("tests/desktop/wdio.feishu-live.conf.mjs", "utf8");
  const sharedConfig = await readFile("tests/desktop/wdio-shared.mjs", "utf8");
  const credentialSpec = await readFile(
    "tests/desktop/specs-feishu-live/credential-safety.e2e.mjs",
    "utf8",
  );
  const operatorSpec = await readFile(
    "tests/desktop/specs-feishu-live/operator-inbound.e2e.mjs",
    "utf8",
  );
  const cleanupSpec = await readFile(
    "tests/desktop/specs-feishu-live/credential-cleanup.e2e.mjs",
    "utf8",
  );
  assert.equal(scripts["test:desktop:feishu-live"], "node scripts/test-desktop.mjs feishu-live");
  assert.match(orchestrator, /mode === "feishu-live"/);
  assert.doesNotMatch(orchestrator, /fullSuiteLayers = \[[^\]]*feishuLiveQualification/);
  assert.match(
    orchestrator,
    /async function feishuLiveQualification\(\)[\s\S]*const artifact = await buildDesktop\(\);[\s\S]*layer: "desktop-feishu-live"[\s\S]*artifact,/,
  );
  assert.match(config, /captureFailureScreenshots: false/);
  assert.match(config, /captureServiceLogs: false/);
  assert.match(config, /logLevel: "silent"/);
  assert.match(config, /commandTimeout: 90_000/);
  assert.match(config, /mochaTimeout: 45 \* 60_000/);
  assert.match(sharedConfig, /"wdio:tauriServiceOptions": \{ commandTimeout \}/);
  assert.match(sharedConfig, /mochaOpts: \{ ui: "bdd", timeout: mochaTimeout \}/);
  assert.match(config, /VANEHUB_FEISHU_APP_SECRET: ""/);
  assert.match(config, /operator-inbound\.e2e\.mjs/);
  assert.match(config, /restart-and-invalid\.e2e\.mjs/);
  assert.match(config, /credential-cleanup\.e2e\.mjs/);
  assert.match(config, /PATH: `\$\{cliFixtureDir\}/);
  assert.doesNotMatch(config, /beforeExit/);
  assert.match(cleanupSpec, /core\.invoke\("clear_im_connector"/);
  assert.match(credentialSpec, /appSecretInput\.setValue\(appSecret\)/);
  assert.match(credentialSpec, /clickConnectorToggle\(false\)/);
  assert.match(credentialSpec, /clickConnectorAction\("test"\)/);
  assert.match(credentialSpec, /waitForOperationSuccess\(\)/);
  assert.match(credentialSpec, /waitForLifecycle\("connected"\)/);
  assert.match(credentialSpec, /view\?\.health\.lifecycle === "failed"/);
  assert.match(credentialSpec, /view\.health\.safeErrorCode/);
  assert.match(credentialSpec, /failed\?\.health\.safeErrorCode/);
  assert.match(credentialSpec, /view\.health\.updatedAt !== previousUpdatedAt/);
  assert.match(credentialSpec, /browser\.waitUntil/);
  assert.doesNotMatch(credentialSpec, /core\.invoke\("save_im_connector"/);
  assert.match(operatorSpec, /VANEHUB_FEISHU_LIVE_OPERATOR/);
  assert.match(operatorSpec, /promoteToMultiAgentSession/);
  assert.doesNotMatch(operatorSpec, /browser\.pause\(90_000\)/);
  assert.match(operatorSpec, /feishu-platform-retry-not-observed/);
  assert.match(operatorSpec, /VANEHUB_E2E_OVERSIZED/);
});
