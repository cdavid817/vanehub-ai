import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import process from "node:process";

mkdirSync("coverage/native", { recursive: true });

const result = spawnSync(
  process.platform === "win32" ? "cargo.exe" : "cargo",
  [
    "llvm-cov",
    "--manifest-path",
    "src-tauri/Cargo.toml",
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
