import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

// The plan is exported so its coverage of the repository's mandatory command list can be
// asserted without executing a multi-hour verification run. Every entry must delegate to an
// existing npm script or toolchain command; this file must never reimplement one of them.
export function repositoryVerificationCommands(npmCli) {
  const npm = (args) => [process.execPath, [npmCli, ...args]];
  return [
    npm(["run", "lint:ci"]),
    npm(["run", "docs:check"]),
    npm(["run", "contracts:check"]),
    npm(["run", "coverage:policy:test"]),
    npm(["run", "version:unit:test"]),
    npm(["run", "release:unit:test"]),
    npm(["run", "test"]),
    npm(["run", "test:coverage"]),
    npm(["run", "build"]),
    ["cargo", ["fmt", "--manifest-path", "src-tauri/Cargo.toml", "--all", "--", "--check"]],
    ["cargo", ["check", "--workspace"]],
    ["cargo", ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]],
    npm(["run", "native:panic:check"]),
    ["cargo", ["test", "--workspace"]],
    npm([
      "exec",
      "--yes",
      "--package=@fission-ai/openspec@1.8.0",
      "--",
      "openspec",
      "validate",
      "--specs",
      "--strict",
    ]),
    npm(["run", "desktop:unit:test"]),
    npm(["run", "test:desktop"]),
  ];
}

function runRepositoryVerification() {
  const npmCli = process.env.npm_execpath;
  if (!npmCli) throw new Error("npm_execpath is required to run repository verification.");
  for (const [command, args] of repositoryVerificationCommands(npmCli)) {
    const result = spawnSync(command, args, { stdio: "inherit" });
    if (result.error || result.status !== 0) {
      process.exitCode = result.status ?? 1;
      break;
    }
  }
}

const entryPoint = process.argv[1] ? pathToFileURL(process.argv[1]).href : "";
if (entryPoint === import.meta.url || process.argv[1] === fileURLToPath(import.meta.url)) {
  runRepositoryVerification();
}
