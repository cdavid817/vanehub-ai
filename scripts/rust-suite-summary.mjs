// Runs a cargo test invocation and prints only its result lines and failure list.
//
// `cargo test`'s own output is far larger than the terminal buffer this harness keeps, so tailing
// it loses exactly the summary that matters. This keeps the whole run and reports the parts a
// reader needs to judge it: every `test result:` line and every named failure.
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const result = spawnSync("cargo", ["test", ...args], { encoding: "utf8", maxBuffer: 1024 * 1024 * 256 });
const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;

const summaries = output.match(/^test result: .*$/gm) ?? [];
const failed = output.match(/^\s{4}\S+::\S+$/gm) ?? [];
const panics = output.match(/^thread .*panicked at [^\n]*\n[^\n]*/gm) ?? [];

process.stdout.write(`${summaries.join("\n")}\n`);
if (failed.length > 0) process.stdout.write(`\nfailures:\n${[...new Set(failed)].join("\n")}\n`);
if (panics.length > 0) {
  process.stdout.write(`\npanics:\n${panics.map((p) => p.replace(/\s+/g, " ").slice(0, 220)).join("\n")}\n`);
}
process.stdout.write(`\nexit status: ${result.status}\n`);
