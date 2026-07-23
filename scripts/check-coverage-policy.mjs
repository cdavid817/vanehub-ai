import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

function normalizePath(filename, root = process.cwd()) {
  const normalizedRoot = path.resolve(root).replaceAll("\\", "/");
  const normalizedFilename = path.resolve(filename).replaceAll("\\", "/");
  if (normalizedFilename === normalizedRoot) return "";
  if (normalizedFilename.startsWith(`${normalizedRoot}/`)) {
    return normalizedFilename.slice(normalizedRoot.length + 1);
  }
  for (const marker of ["/src-tauri/", "/src/"]) {
    const index = normalizedFilename.lastIndexOf(marker);
    if (index >= 0) return normalizedFilename.slice(index + 1);
  }
  return filename.replaceAll("\\", "/").replace(/^\.\//, "");
}

function globRegex(pattern) {
  const normalized = pattern.replaceAll("\\", "/");
  let expression = "^";
  for (let index = 0; index < normalized.length; index += 1) {
    const character = normalized[index];
    if (character === "*" && normalized[index + 1] === "*") {
      expression += ".*";
      index += 1;
    } else if (character === "*") {
      expression += "[^/]*";
    } else if (character === "?") {
      expression += "[^/]";
    } else {
      expression += character.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
    }
  }
  return new RegExp(`${expression}$`);
}

function percentage(covered, total) {
  if (!Number.isFinite(covered) || !Number.isFinite(total) || covered < 0 || total < 0) {
    throw new Error("Coverage report contains invalid line totals.");
  }
  return total === 0 ? 100 : (covered / total) * 100;
}

export function parseFrontendReport(report, root = process.cwd()) {
  if (!report || typeof report !== "object" || !report.total?.lines) {
    throw new Error("Frontend coverage report is malformed or incomplete.");
  }
  const files = Object.entries(report)
    .filter(([filename]) => filename !== "total")
    .map(([filename, summary]) => {
      if (!summary || typeof summary !== "object" || !summary.lines) {
        throw new Error(`Frontend coverage entry is incomplete: ${filename}`);
      }
      return {
        path: normalizePath(filename, root),
        covered: Number(summary.lines.covered),
        total: Number(summary.lines.total),
      };
    });
  if (files.length === 0) throw new Error("Frontend coverage report contains no production files.");
  return {
    files,
    total: {
      covered: Number(report.total.lines.covered),
      total: Number(report.total.lines.total),
    },
  };
}

export function parseNativeReport(report, root = process.cwd()) {
  if (!report || typeof report !== "object" || !Array.isArray(report.data) || report.data.length === 0) {
    throw new Error("Native coverage report is malformed or incomplete.");
  }
  const byPath = new Map();
  for (const dataset of report.data) {
    if (!dataset || !Array.isArray(dataset.files)) {
      throw new Error("Native coverage dataset is missing file entries.");
    }
    for (const file of dataset.files) {
      const lines = file?.summary?.lines;
      if (typeof file?.filename !== "string" || !lines) {
        throw new Error("Native coverage file entry is malformed.");
      }
      const normalized = normalizePath(file.filename, root);
      const current = byPath.get(normalized) ?? { path: normalized, covered: 0, total: 0 };
      current.covered += Number(lines.covered);
      current.total += Number(lines.count);
      byPath.set(normalized, current);
    }
  }
  const files = [...byPath.values()].filter((file) => file.path.startsWith("src-tauri/src/"));
  if (files.length === 0) throw new Error("Native coverage report contains no src-tauri production files.");
  return {
    files,
    total: files.reduce(
      (total, file) => ({ covered: total.covered + file.covered, total: total.total + file.total }),
      { covered: 0, total: 0 },
    ),
  };
}

function groupCoverage(files, patterns) {
  const matchers = patterns.map(globRegex);
  const matches = files.filter((file) => matchers.some((matcher) => matcher.test(file.path)));
  if (matches.length === 0) {
    throw new Error(`Coverage policy group matched no production files: ${patterns.join(", ")}`);
  }
  return matches.reduce(
    (total, file) => ({ covered: total.covered + file.covered, total: total.total + file.total }),
    { covered: 0, total: 0 },
  );
}

function formatPercent(value) {
  return `${value.toFixed(2)}%`;
}

export function evaluateCoverage(kind, parsed, policy) {
  const selected = policy?.[kind];
  if (!selected || !Number.isFinite(selected.minimumLines)) {
    throw new Error(`Coverage policy is missing ${kind}.minimumLines.`);
  }
  const rows = [];
  const failures = [];
  const totalPercent = percentage(parsed.total.covered, parsed.total.total);
  rows.push({ name: `${kind}-total`, percent: totalPercent, minimum: selected.minimumLines });
  if (totalPercent + Number.EPSILON < selected.minimumLines) {
    failures.push(`${kind} total ${formatPercent(totalPercent)} is below ${formatPercent(selected.minimumLines)}`);
  }
  if (kind === "native") {
    if (!Array.isArray(selected.criticalGroups) || selected.criticalGroups.length === 0) {
      throw new Error("Coverage policy defines no native critical groups.");
    }
    for (const group of selected.criticalGroups) {
      if (!group?.name || !Array.isArray(group.patterns) || group.patterns.length === 0 || !Number.isFinite(group.minimumLines)) {
        throw new Error("Native critical coverage group is malformed.");
      }
      const totals = groupCoverage(parsed.files, group.patterns);
      const percent = percentage(totals.covered, totals.total);
      rows.push({ name: group.name, percent, minimum: group.minimumLines });
      if (percent + Number.EPSILON < group.minimumLines) {
        failures.push(`${group.name} ${formatPercent(percent)} is below ${formatPercent(group.minimumLines)}`);
      }
    }
  }
  return { rows, failures };
}

export function renderSummary(rows) {
  const lines = [
    "| Coverage group | Lines | Required |",
    "| --- | ---: | ---: |",
    ...rows.map((row) => `| ${row.name} | ${formatPercent(row.percent)} | ${formatPercent(row.minimum)} |`),
  ];
  return lines.join("\n");
}

function parseArguments(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) throw new Error(`Invalid argument: ${key ?? ""}`);
    values.set(key.slice(2), value);
  }
  const kind = values.get("kind");
  if (kind !== "frontend" && kind !== "native") throw new Error("--kind must be frontend or native.");
  const report = values.get("report");
  if (!report) throw new Error("--report is required.");
  return { kind, report, policy: values.get("policy") ?? "coverage-policy.json" };
}

function main() {
  const options = parseArguments(process.argv.slice(2));
  const policy = JSON.parse(readFileSync(options.policy, "utf8"));
  const report = JSON.parse(readFileSync(options.report, "utf8"));
  const parsed = options.kind === "frontend" ? parseFrontendReport(report) : parseNativeReport(report);
  const result = evaluateCoverage(options.kind, parsed, policy);
  const summary = renderSummary(result.rows);
  writeFileSync(
    path.join(path.dirname(options.report), "policy-summary.md"),
    `# ${options.kind} coverage\n\n${summary}\n`,
  );
  process.stdout.write(`${summary}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## ${options.kind} coverage\n\n${summary}\n\n`);
  }
  if (result.failures.length > 0) {
    throw new Error(`Coverage policy failed:\n- ${result.failures.join("\n- ")}`);
  }
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
