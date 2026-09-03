// A diagnostic, not a gate.
//
// Runs one known-flaky test N times on the current HEAD and reports the distribution of outcomes
// with the exact error each failure produced. Nothing here retries until green, relaxes an
// assertion, or sleeps: the point is to have a comparable "before" and "after" reading so a change
// that made concurrency worse can be told apart from the noise that was already there.
import { spawnSync } from "node:child_process";

const RUNS = Number(process.argv[2] ?? 5);
const TEST = process.argv[3] ?? "concurrent_operation_writers_remain_bounded_and_durable";

const outcomes = [];
for (let run = 1; run <= RUNS; run += 1) {
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--lib",
      "--",
      "--test-threads=1",
      TEST,
    ],
    { encoding: "utf8", shell: false },
  );
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  const summary = output.match(/^test result: .*$/m)?.[0] ?? "no summary";
  const error =
    output.match(/panicked at [^\n]*\n[^\n]*/)?.[0]?.replace(/\s+/g, " ").slice(0, 200) ?? null;
  const passed = /^test result: ok\./m.test(output);
  outcomes.push({ run, passed, summary, error });
  process.stdout.write(`run ${run}: ${passed ? "PASS" : "FAIL"} — ${summary}\n`);
  if (error) process.stdout.write(`         ${error}\n`);
}

const passes = outcomes.filter((outcome) => outcome.passed).length;
process.stdout.write(`\n${TEST}\n${passes}/${RUNS} passed, ${RUNS - passes} failed\n`);
