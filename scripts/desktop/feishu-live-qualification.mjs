import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { createLayerResult } from "./result.mjs";

export const FEISHU_LIVE_OPT_IN = "VANEHUB_FEISHU_LIVE_QUALIFICATION";

const prerequisites = [
  { code: "dedicated-tenant", environment: "VANEHUB_FEISHU_TEST_TENANT" },
  { code: "app-id", environment: "VANEHUB_FEISHU_APP_ID" },
  { code: "app-secret", environment: "VANEHUB_FEISHU_APP_SECRET" },
  {
    code: "bot-message-permissions",
    environment: "VANEHUB_FEISHU_PERMISSIONS_CONFIRMED",
    expected: "1",
  },
  {
    code: "long-connection-subscription",
    environment: "VANEHUB_FEISHU_LONG_CONNECTION_CONFIRMED",
    expected: "1",
  },
  { code: "direct-test-chat", environment: "VANEHUB_FEISHU_TEST_CHAT_ID" },
];

function isPresent(requirement, env) {
  const value = env[requirement.environment];
  if (requirement.expected) return value === requirement.expected;
  return typeof value === "string" && value.trim().length > 0;
}

export function evaluateFeishuLivePrerequisites(env = process.env) {
  const optedIn = env[FEISHU_LIVE_OPT_IN] === "1";
  const checks = prerequisites.map((requirement) => ({
    code: requirement.code,
    present: isPresent(requirement, env),
  }));
  if (!optedIn) {
    return {
      status: "NOT RUN",
      reason: "explicit-opt-in-required",
      optedIn: false,
      prerequisites: checks,
    };
  }
  const missingPrerequisites = checks.filter(({ present }) => !present).map(({ code }) => code);
  if (missingPrerequisites.length > 0) {
    return {
      status: "BLOCKED",
      reason: "live-prerequisites-unavailable",
      optedIn: true,
      prerequisites: checks,
      missingPrerequisites,
    };
  }
  return {
    status: "READY",
    reason: "live-prerequisites-ready",
    optedIn: true,
    prerequisites: checks,
  };
}

export async function writeFeishuLivePreflight(repoRoot, env = process.env, now = new Date()) {
  const preflight = evaluateFeishuLivePrerequisites(env);
  const runId = `${now.toISOString().replaceAll(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`;
  const resultDir = path.join(repoRoot, "test-results", "desktop-live", runId);
  await mkdir(resultDir, { recursive: true });
  const status = preflight.status === "READY" ? "NOT RUN" : preflight.status;
  const reason = preflight.status === "READY"
    ? "live-qualification-execution-pending"
    : preflight.reason;
  const layer = createLayerResult({
    layer: "desktop-feishu-live",
    status,
    reason,
    livePlatform: true,
    fixture: false,
    observedAt: now.toISOString(),
    preflight,
  });
  const summaryPath = path.join(resultDir, "summary.json");
  await writeFile(summaryPath, `${JSON.stringify({ layers: [layer] }, null, 2)}\n`);
  return { status, reason, preflight, resultDir, summaryPath };
}
