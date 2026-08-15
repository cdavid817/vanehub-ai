import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { repositoryVerificationCommands } from "./test-verify.mjs";

// Placed under the desktop-* glob because `npm run desktop:unit:test` is the only unit-test
// entry point that runs on all three CI platforms, and `test:verify` is the command that
// composes desktop verification with the rest of the mandatory list.
const NPM_CLI = "/stub/npm-cli.js";

// Commands that AGENTS.md marks as conditionally mandatory rather than listing in the main
// fenced block. They are enumerated here because no machine-readable source declares them.
const CONDITIONAL_COMMANDS = [
  "npm run test:coverage",
  "npm run coverage:policy:test",
  "npm run version:unit:test",
  "npm run contracts:check",
  "npm run docs:check",
  "npm run desktop:unit:test",
  "npm run test:desktop",
];

function canonicalize([command, args]) {
  if (command !== process.execPath || args[0] !== NPM_CLI) return [command, ...args].join(" ");
  const npmArgs = args.slice(1);
  if (npmArgs[0] === "run") return `npm run ${npmArgs[1]}`;
  const separator = npmArgs.indexOf("--");
  return npmArgs.slice(separator + 1).join(" ");
}

async function mandatoryCommandsFromAgentsDoc() {
  const doc = (await readFile("AGENTS.md", "utf8")).replaceAll("\r\n", "\n");
  const block = [...doc.matchAll(/```bash\n([\s\S]*?)```/g)]
    .map((match) => match[1])
    .find((body) => body.includes("cargo fmt --manifest-path"));
  assert.ok(block, "AGENTS.md must keep the mandatory verification commands in a bash block.");
  return block.split("\n").map((line) => line.trim()).filter(Boolean);
}

test("test:verify orchestrates every command AGENTS.md marks mandatory", async () => {
  const plan = repositoryVerificationCommands(NPM_CLI).map(canonicalize);
  const mandatory = await mandatoryCommandsFromAgentsDoc();

  assert.ok(mandatory.length >= 8, "The mandatory command block was parsed incorrectly.");
  for (const command of mandatory) {
    assert.ok(plan.includes(command), `test:verify must run "${command}".`);
  }
  for (const command of CONDITIONAL_COMMANDS) {
    assert.ok(plan.includes(command), `test:verify must run "${command}".`);
  }
});

test("test:verify delegates to existing npm scripts instead of reimplementing them", async () => {
  const { scripts } = JSON.parse(await readFile("package.json", "utf8"));
  const plan = repositoryVerificationCommands(NPM_CLI);

  for (const entry of plan) {
    const canonical = canonicalize(entry);
    if (!canonical.startsWith("npm run ")) continue;
    assert.ok(canonical.slice("npm run ".length) in scripts, `Unknown npm script in ${canonical}.`);
  }
  assert.equal(new Set(plan.map(canonicalize)).size, plan.length);
  assert.equal(canonicalize(plan.at(-1)), "npm run test:desktop");
});
