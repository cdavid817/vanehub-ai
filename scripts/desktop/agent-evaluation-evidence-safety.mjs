import { Buffer } from "node:buffer";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

const REPORT_NAME = "agent-evaluation-evidence-safety.json";
const TEXT_EXTENSIONS = new Set([".json", ".log", ".txt"]);
const IMAGE_EXTENSIONS = new Set([".bmp", ".gif", ".jpeg", ".jpg", ".png", ".webp"]);
const MAX_SCANNED_BYTES = 8 * 1024 * 1024;

async function listFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const candidate = path.join(directory, entry.name);
    return entry.isDirectory() ? listFiles(candidate) : [candidate];
  }));
  return nested.flat();
}

export async function auditAgentEvaluationEvidence(resultDir, env = process.env) {
  const secretValues = [env.VANEHUB_ONEPIECE_API_KEY]
    .filter((value) => typeof value === "string" && value.length >= 8);
  const files = (await listFiles(resultDir))
    .filter((file) => path.basename(file) !== REPORT_NAME)
    .sort((left, right) => left.localeCompare(right));
  const findings = [];
  let scannedBytes = 0;

  for (const file of files) {
    const relative = path.relative(resultDir, file);
    const extension = path.extname(file).toLowerCase();
    if (IMAGE_EXTENSIONS.has(extension)) {
      findings.push({ file: relative, rule: "screenshot-content-retained" });
      continue;
    }
    if (!TEXT_EXTENSIONS.has(extension)) continue;
    const contents = await readFile(file, "utf8");
    scannedBytes += Buffer.byteLength(contents);
    if (scannedBytes > MAX_SCANNED_BYTES) {
      findings.push({ file: relative, rule: "evidence-scan-budget-exceeded" });
      break;
    }
    if (secretValues.some((value) => contents.includes(value))) {
      findings.push({ file: relative, rule: "provider-secret-retained" });
    }
  }

  const report = {
    status: findings.length === 0 ? "PASSED" : "FAILED",
    policy: "agent-evaluation-secret-safety-v1",
    scannedFiles: files.length,
    scannedBytes,
    findings,
  };
  await writeFile(path.join(resultDir, REPORT_NAME), `${JSON.stringify(report, null, 2)}\n`);
  return report;
}
