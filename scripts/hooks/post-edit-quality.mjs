// Claude Code PostToolUse hook (registered in .claude/settings.json).
// After Edit/Write on a TypeScript file, runs eslint --fix and feeds residual
// errors back to the agent; after a Rust file, runs rustfmt. Exit code 2 makes
// the harness surface stderr to the agent as blocking feedback; anything else
// passes through. Fail-open by design: infrastructure failures (missing
// toolchain, malformed payload, eslint crash) never block an edit — the
// pre-commit hooks and CI remain the enforcing gates.
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
// Must match the edition in src-tauri/Cargo.toml.
const RUST_EDITION = "2021";
const GENERATED_DIRS =
  /(^|[\\/])(node_modules|dist|target|coverage|\.docs-build|\.docs-target)([\\/]|$)/;

function blockingFeedback(message) {
  process.stderr.write(message);
  process.exit(2);
}

let filePath = "";
let baseDir = repoRoot;
try {
  const payload = JSON.parse(readFileSync(0, "utf8"));
  filePath = payload?.tool_input?.file_path ?? "";
  if (typeof payload?.cwd === "string" && payload.cwd) baseDir = payload.cwd;
} catch {
  process.exit(0);
}
if (!filePath) process.exit(0);

const abs = isAbsolute(filePath) ? filePath : resolve(baseDir, filePath);
const rel = relative(repoRoot, abs);
if (rel.startsWith("..") || GENERATED_DIRS.test(rel)) process.exit(0);

if (/\.(ts|tsx|mts|cts)$/.test(abs)) {
  const eslintBin = join(repoRoot, "node_modules", "eslint", "bin", "eslint.js");
  if (!existsSync(eslintBin)) process.exit(0);
  const result = spawnSync(
    process.execPath,
    [eslintBin, "--fix", "--no-warn-ignored", "--max-warnings=0", abs],
    { cwd: repoRoot, encoding: "utf8", timeout: 75_000 },
  );
  // status 1 = lint problems; other failures are infrastructure and fail open.
  if (result.status === 1) {
    blockingFeedback(
      `ESLint 在 ${rel} 自动修复后仍有错误,请修复后再继续(禁 any/@ts-ignore,单文件不超过 300 行):\n` +
        (result.stdout || result.stderr || "").slice(0, 4000),
    );
  }
  process.exit(0);
}

if (abs.endsWith(".rs")) {
  const result = spawnSync("rustfmt", ["--edition", RUST_EDITION, abs], {
    cwd: repoRoot,
    encoding: "utf8",
    timeout: 30_000,
  });
  // Missing toolchain or timeout: fail open.
  if (result.error || result.status === null) process.exit(0);
  if (result.status !== 0) {
    blockingFeedback(
      `rustfmt 无法格式化 ${rel}(通常意味着语法错误),请修复后再继续:\n` +
        (result.stderr || "").slice(0, 4000),
    );
  }
  process.exit(0);
}

process.exit(0);
