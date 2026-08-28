import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const fixtureEventsPath = path.join(sourceRoot, "tests", "desktop", "fixtures", "feishu", "events.json");
const REPORT_NAME = "feishu-evidence-safety.json";
const TEXT_EXTENSIONS = new Set([".json", ".log", ".txt"]);
const IMAGE_EXTENSIONS = new Set([".bmp", ".gif", ".jpeg", ".jpg", ".png", ".webp"]);
const FIXED_PRIVATE_VALUES = [
  "desktop-e2e-feishu-chat-v1",
  "desktop-e2e-feishu-sender-v1",
  "evt-fixture-001",
  "ou_fixture_sender",
  "om_fixture_message",
  "oc_fixture_chat",
  "status please",
  "deterministic fixture final",
  "VANEHUB_E2E_OVERSIZED",
  "VANEHUB_E2E_SLOW",
  "界界界界",
];
const UNSAFE_PATTERNS = [
  ["raw-feishu-protocol", /im\.message\.receive_v1|"sender_id"|"chat_id"/iu],
  [
    "unredacted-credential",
    /(?:app[_-]?secret|client[_-]?secret|api[_-]?key|access[_-]?token|refresh[_-]?token|authorization|credential)\s*[=:]\s*(?!\[REDACTED\]|null\b|none\b)[^\s,}\]]+/iu,
  ],
  ["bearer-token", /bearer\s+(?!\[REDACTED\])\S+/iu],
];

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
  if (normalized === "feishu-fixture-ledger.json") return "fixtureLedger";
  if (normalized.endsWith(".json")) return "resultJson";
  return "other";
}

function recordFinding(findings, file, category, rule) {
  findings.push({ file, category, rule });
}

export async function auditFeishuEvidence(resultDir) {
  const events = JSON.parse(await readFile(fixtureEventsPath, "utf8"));
  const privateValues = [
    ...FIXED_PRIVATE_VALUES,
    ...Object.values(events).flatMap((event) => [event.eventId, event.text]),
  ].filter((value) => typeof value === "string" && value.length > 0);
  const files = (await listFiles(resultDir))
    .filter((file) => path.basename(file) !== REPORT_NAME)
    .sort((left, right) => left.localeCompare(right));
  const counts = {
    fixtureLedger: 0,
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
    counts[category] += 1;
    const extension = path.extname(file).toLowerCase();
    if (IMAGE_EXTENSIONS.has(extension)) {
      recordFinding(findings, relative, category, "screenshot-content-not-retained");
      continue;
    }
    if (!TEXT_EXTENSIONS.has(extension)) continue;
    const contents = await readFile(file, "utf8");
    if (privateValues.some((value) => contents.includes(value))) {
      recordFinding(findings, relative, category, "private-fixture-value");
    }
    for (const [rule, pattern] of UNSAFE_PATTERNS) {
      if (pattern.test(contents)) recordFinding(findings, relative, category, rule);
    }
  }

  if (counts.fixtureLedger !== 1) {
    recordFinding(findings, "feishu-fixture-ledger.json", "fixtureLedger", "ledger-evidence-missing");
  }
  const report = {
    status: findings.length === 0 ? "PASSED" : "FAILED",
    policy: "feishu-evidence-safe-metadata-v1",
    scannedFiles: files.length,
    categories: counts,
    findings,
  };
  await mkdir(resultDir, { recursive: true });
  await writeFile(path.join(resultDir, REPORT_NAME), `${JSON.stringify(report, null, 2)}\n`);
  return report;
}
