import { spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { createLayerResult } from "./result.mjs";

export const AGENT_EVALUATION_MODES = Object.freeze([
  "fixture-opencode",
  "live-opencode",
  "live-onepiece",
]);

const ANSI_PATTERN = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, "g");
const stripAnsi = (value) => value.replaceAll(ANSI_PATTERN, "");

export function hasOpenCodeCredentials(output) {
  const normalized = stripAnsi(output).trim();
  return normalized.length > 0 && !/\b0 credentials\b/iu.test(normalized);
}

function defaultProbe(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 30_000,
    windowsHide: true,
  });
  return {
    ok: !result.error && result.status === 0,
    output: `${result.stdout ?? ""}\n${result.stderr ?? ""}`,
  };
}

export function evaluateAgentEvaluationPrerequisites({
  mode,
  env = process.env,
  platform = process.platform,
  probe = defaultProbe,
}) {
  if (!AGENT_EVALUATION_MODES.includes(mode)) {
    return { status: "BLOCKED", reason: "unsupported-evaluation-mode", mode };
  }
  if (mode === "fixture-opencode") {
    return { status: "READY", reason: "fixture-opencode-ready", mode, fixture: true };
  }
  if (mode === "live-onepiece") {
    const direct = typeof env.VANEHUB_ONEPIECE_API_KEY === "string"
      && env.VANEHUB_ONEPIECE_API_KEY.trim().length > 0;
    const profile = platform === "win32"
      && typeof env.VANEHUB_ONEPIECE_PROFILE_ID === "string"
      && env.VANEHUB_ONEPIECE_PROFILE_ID.trim().length > 0;
    return direct || profile
      ? { status: "READY", reason: "onepiece-credential-ready", mode, fixture: false }
      : {
          status: "BLOCKED",
          reason: "onepiece-credential-unavailable",
          mode,
          fixture: false,
          missingPrerequisites: ["onepiece-credential"],
        };
  }

  const installed = probe("opencode", ["--version"]);
  const authentication = installed.ok ? probe("opencode", ["auth", "list"]) : { ok: false, output: "" };
  const authenticated = authentication.ok && hasOpenCodeCredentials(authentication.output);
  const missingPrerequisites = [
    ...(!installed.ok ? ["opencode-executable"] : []),
    ...(installed.ok && !authenticated ? ["opencode-authentication"] : []),
  ];
  return missingPrerequisites.length === 0
    ? { status: "READY", reason: "live-opencode-ready", mode, fixture: false }
    : {
        status: "BLOCKED",
        reason: "live-opencode-prerequisites-unavailable",
        mode,
        fixture: false,
        missingPrerequisites,
      };
}

export async function writeAgentEvaluationPreflight(repoRoot, preflight, now = new Date()) {
  const runId = `${now.toISOString().replaceAll(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`;
  const resultDir = path.join(repoRoot, "test-results", "desktop-live", runId);
  await mkdir(resultDir, { recursive: true });
  const status = preflight.status === "READY" ? "NOT RUN" : preflight.status;
  const layer = createLayerResult({
    layer: "desktop-agent-evaluation",
    status,
    reason: preflight.status === "READY" ? "evaluation-execution-pending" : preflight.reason,
    mode: preflight.mode,
    fixture: preflight.fixture,
    observedAt: now.toISOString(),
    prerequisites: preflight.missingPrerequisites ?? [],
  });
  const summaryPath = path.join(resultDir, "summary.json");
  await writeFile(summaryPath, `${JSON.stringify({ layers: [layer] }, null, 2)}\n`);
  return { status, resultDir, summaryPath };
}
