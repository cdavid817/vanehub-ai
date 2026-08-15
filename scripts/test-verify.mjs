import { spawnSync } from "node:child_process";

const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error("npm_execpath is required to run repository verification.");
const npm = (args) => [process.execPath, [npmCli, ...args]];
const commands = [
  npm(["run", "lint:ci"]),
  npm(["run", "test"]),
  npm(["run", "build"]),
  ["cargo", ["fmt", "--manifest-path", "src-tauri/Cargo.toml", "--all", "--", "--check"]],
  ["cargo", ["clippy", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets", "--", "-D", "warnings"]],
  ["cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml"]],
  ["cargo", ["check", "--manifest-path", "src-tauri/Cargo.toml"]],
  npm([
    "exec",
    "--yes",
    "--package=@fission-ai/openspec@1.6.0",
    "--",
    "openspec",
    "validate",
    "--specs",
    "--strict",
  ]),
  npm(["run", "test:desktop"]),
];

for (const [command, args] of commands) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.error || result.status !== 0) {
    process.exitCode = result.status ?? 1;
    break;
  }
}
