import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

const MAX_LOG_BYTES = 1024 * 1024;

async function copyBounded(source, destination) {
  const contents = await readFile(source);
  const bounded = contents.length > MAX_LOG_BYTES ? contents.subarray(contents.length - MAX_LOG_BYTES) : contents;
  await writeFile(destination, bounded);
  return { source, destination, bytes: bounded.length, truncated: bounded.length !== contents.length };
}

export async function collectUnifiedLogs(dataDir, resultDir) {
  const sourceDir = path.join(dataDir, "logs");
  const destinationDir = path.join(resultDir, "logs", "native");
  await mkdir(destinationDir, { recursive: true });
  let entries;
  try {
    entries = await readdir(sourceDir, { withFileTypes: true });
  } catch (error) {
    return { files: [], unavailable: [`Unified log directory unavailable: ${error.message}`] };
  }
  const files = [];
  const unavailable = [];
  for (const entry of entries.filter((candidate) => candidate.isFile() && candidate.name.endsWith(".log"))) {
    try {
      files.push(await copyBounded(path.join(sourceDir, entry.name), path.join(destinationDir, entry.name)));
    } catch (error) {
      unavailable.push(`${entry.name}: ${error.message}`);
    }
  }
  return { files, unavailable };
}

export async function writeRunSummary(resultDir, summary) {
  await mkdir(resultDir, { recursive: true });
  const summaryPath = path.join(resultDir, "summary.json");
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  return summaryPath;
}
