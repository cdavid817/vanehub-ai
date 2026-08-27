import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

const REPORT_NAME = "feishu-live-evidence-safety.json";
const TEXT_EXTENSIONS = new Set([".json", ".log", ".txt"]);
const IMAGE_EXTENSIONS = new Set([".bmp", ".gif", ".jpeg", ".jpg", ".png", ".webp"]);
const PRIVATE_ENVIRONMENT_KEYS = [
  "VANEHUB_FEISHU_TEST_TENANT",
  "VANEHUB_FEISHU_APP_ID",
  "VANEHUB_FEISHU_APP_SECRET",
  "VANEHUB_FEISHU_TEST_CHAT_ID",
];
const PRIVATE_MESSAGE_VALUES = [
  "VANEHUB_LIVE_DIRECT_CHECK",
  "VANEHUB_LIVE_SINGLE_CONFIRMED",
  "VANEHUB_E2E_OVERSIZED",
  "VANEHUB_LIVE_CHUNK_CONFIRMED",
  "live mentioned route",
  "VANEHUB_LIVE_DEFAULT_CHECK",
  "live invalid route",
  "VANEHUB_LIVE_INVALID_CONFIRMED",
  "VANEHUB_LIVE_DISABLED_CHECK",
  "VANEHUB_LIVE_REENABLE_CONFIRMED",
  "VANEHUB_LIVE_RESTART_CHECK",
];
const EXTERNAL_ID_PATTERN = /\b(?:cli|oc|om|ou)_[a-z0-9]{12,}\b/iu;
const PAIRING_COMMAND_PATTERN = /\/bind\s+[a-z0-9]{4,}/iu;

async function listFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const candidate = path.join(directory, entry.name);
    return entry.isDirectory() ? listFiles(candidate) : [candidate];
  }));
  return nested.flat();
}

function categoryFor(relativePath) {
  const normalized = relativePath.replaceAll("\\", "/");
  if (normalized.startsWith("screenshots/")) return "screenshots";
  if (normalized.startsWith("logs/native/")) return "unifiedLogs";
  if (normalized.startsWith("logs/")) return "wdioDiagnostics";
  if (normalized === "feishu-live-credential-cleanup.json") return "credentialCleanup";
  if (normalized.endsWith(".json")) return "resultJson";
  return "other";
}

export async function auditFeishuLiveEvidence(resultDir, env = process.env) {
  const privateValues = PRIVATE_ENVIRONMENT_KEYS
    .map((key) => env[key])
    .filter((value) => typeof value === "string" && value.length > 0);
  const files = (await listFiles(resultDir))
    .filter((file) => path.basename(file) !== REPORT_NAME)
    .sort((left, right) => left.localeCompare(right));
  const categories = {
    credentialCleanup: 0,
    resultJson: 0,
    screenshots: 0,
    unifiedLogs: 0,
    wdioDiagnostics: 0,
    other: 0,
  };
  const findings = [];

  for (const file of files) {
    const relative = path.relative(resultDir, file);
    const category = categoryFor(relative);
    categories[category] += 1;
    const extension = path.extname(file).toLowerCase();
    if (IMAGE_EXTENSIONS.has(extension)) {
      findings.push({ file: relative, category, rule: "screenshot-content-not-retained" });
      continue;
    }
    if (!TEXT_EXTENSIONS.has(extension)) continue;
    const contents = await readFile(file, "utf8");
    if (privateValues.some((value) => contents.includes(value))) {
      findings.push({ file: relative, category, rule: "runtime-private-value" });
    }
    if (PRIVATE_MESSAGE_VALUES.some((value) => contents.includes(value))) {
      findings.push({ file: relative, category, rule: "message-content-retained" });
    }
    if (EXTERNAL_ID_PATTERN.test(contents)) {
      findings.push({ file: relative, category, rule: "external-identifier-retained" });
    }
    if (PAIRING_COMMAND_PATTERN.test(contents)) {
      findings.push({ file: relative, category, rule: "pairing-command-retained" });
    }
  }

  const cleanupPath = path.join(resultDir, "feishu-live-credential-cleanup.json");
  try {
    const cleanup = JSON.parse(await readFile(cleanupPath, "utf8"));
    if (cleanup.status !== "CLEARED" || cleanup.credentialProfileOwned !== true) {
      findings.push({
        file: "feishu-live-credential-cleanup.json",
        category: "credentialCleanup",
        rule: "run-owned-credential-not-cleared",
      });
    }
  } catch {
    findings.push({
      file: "feishu-live-credential-cleanup.json",
      category: "credentialCleanup",
      rule: "credential-cleanup-evidence-missing",
    });
  }

  const report = {
    status: findings.length === 0 ? "PASSED" : "FAILED",
    policy: "feishu-live-runtime-secrets-v1",
    scannedFiles: files.length,
    categories,
    findings,
  };
  await writeFile(path.join(resultDir, REPORT_NAME), `${JSON.stringify(report, null, 2)}\n`);
  return report;
}
