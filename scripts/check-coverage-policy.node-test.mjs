import assert from "node:assert/strict";
import test from "node:test";
import { evaluateCoverage, parseFrontendReport, parseNativeReport } from "./check-coverage-policy.mjs";

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
