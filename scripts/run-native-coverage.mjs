import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import process from "node:process";

mkdirSync("coverage/native", { recursive: true });

const result = spawnSync(
  process.platform === "win32" ? "cargo.exe" : "cargo",
  [
    "llvm-cov",
    // --workspace, not --manifest-path src-tauri/Cargo.toml: establish-cargo-workspace-skeleton
    // added a second member (vanehub-permission-hook), and --manifest-path alone selects only the
    // package it points at, which would silently drop that crate's real test coverage from the
    // report instead of reporting it as zero.
    "--workspace",
    "--all-targets",
    "--json",
    "--output-path",
    "coverage/native/coverage.json",
  ],
  { stdio: "inherit" },
);

if (result.error) {
  process.stderr.write(`${result.error.message}\n`);
  process.exitCode = 1;
} else {
  process.exitCode = result.status ?? 1;
}
