import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { evaluateCoverage, globRegex, parseFrontendReport, parseNativeReport } from "./check-coverage-policy.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const frontendPolicy = { frontend: { minimumLines: 80 } };
const nativePolicy = {
  native: {
    minimumLines: 50,
    criticalGroups: [
      {
        name: "critical",
        minimumLines: 80,
        patterns: ["src-tauri/src/critical/*.rs"],
      },
    ],
  },
};

test("rejects a wider frontend baseline regression", () => {
  const parsed = parseFrontendReport({
    total: { lines: { covered: 7, total: 10 } },
    "C:\\repo\\src\\feature.ts": { lines: { covered: 7, total: 10 } },
  }, "C:\\repo");
  const result = evaluateCoverage("frontend", parsed, frontendPolicy);
  assert.match(result.failures.join("\n"), /frontend total 70\.00% is below 80\.00%/);
});

test("rejects a native critical group below eighty percent", () => {
  const parsed = parseNativeReport(nativeReport("C:\\repo\\src-tauri\\src\\critical\\path.rs", 7, 10), "C:\\repo");
  const result = evaluateCoverage("native", parsed, nativePolicy);
  assert.match(result.failures.join("\n"), /critical 70\.00% is below 80\.00%/);
});

test("rejects an empty native critical group", () => {
  const parsed = parseNativeReport(nativeReport("/repo/src-tauri/src/other/path.rs", 10, 10), "/repo");
  assert.throws(
    () => evaluateCoverage("native", parsed, nativePolicy),
    /matched no production files/,
  );
});

test("rejects malformed and incomplete reports", () => {
  assert.throws(() => parseFrontendReport({ total: {} }), /malformed or incomplete/);
  assert.throws(() => parseNativeReport({ data: [{}] }), /missing file entries/);
});

test("normalizes Windows and Linux paths into one native policy", () => {
  const windows = parseNativeReport(nativeReport("C:\\repo\\src-tauri\\src\\critical\\path.rs", 9, 10), "C:\\repo");
  const linux = parseNativeReport(nativeReport("/repo/src-tauri/src/critical/path.rs", 9, 10), "/repo");
  assert.equal(evaluateCoverage("native", windows, nativePolicy).failures.length, 0);
  assert.equal(evaluateCoverage("native", linux, nativePolicy).failures.length, 0);
});

/**
 * The empty-group guard above cannot catch a single dead pattern, because `groupCoverage` matches
 * with `some`: one stale entry beside two live ones leaves the group non-empty and silently stops
 * contributing. That is exactly how `src-tauri/src/platform/database/migrations.rs` survived the
 * move into a `migrations/` directory while its 80% gate went on reporting a pass.
 */
test("every pattern in the committed policy still matches a source file", () => {
  const policy = JSON.parse(readFileSync(resolve(repositoryRoot, "coverage-policy.json"), "utf8"));
  const sources = rustSources(resolve(repositoryRoot, "src-tauri", "src"));
  assert.ok(sources.length > 0, "found no Rust sources to match the policy against");
  for (const group of policy.native.criticalGroups) {
    for (const pattern of group.patterns) {
      const matcher = globRegex(pattern);
      assert.ok(
        sources.some((source) => matcher.test(source)),
        `coverage-policy.json group "${group.name}" has a pattern matching no source file: ${pattern}`,
      );
    }
  }
});

function rustSources(directory, prefix = "src-tauri/src") {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? rustSources(resolve(directory, entry.name), `${prefix}/${entry.name}`)
      : entry.name.endsWith(".rs")
        ? [`${prefix}/${entry.name}`]
        : [],
  );
}

function nativeReport(filename, covered, count) {
  return {
    data: [
      {
        files: [
          {
            filename,
            summary: { lines: { covered, count, percent: count === 0 ? 100 : (covered / count) * 100 } },
          },
        ],
      },
    ],
  };
}
