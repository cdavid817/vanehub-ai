import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

export const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const toolchain = JSON.parse(
  readFileSync(resolve(repositoryRoot, "docs", "toolchain.json"), "utf8"),
);

export const mdbookVersion = toolchain.mdbook;

export function run(command, args, options = {}) {
  const executable = process.platform === "win32" && command === "npx" ? "npx.cmd" : command;
  const result = spawnSync(executable, args, {
    cwd: repositoryRoot,
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    ...options,
  });

  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = options.capture
      ? `\n${result.stdout ?? ""}${result.stderr ?? ""}`.trimEnd()
      : "";
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}.${detail}`);
  }
  return result;
}

export function verifyMdbook() {
  const result = run("mdbook", ["--version"], { capture: true });
  const versionOutput = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  if (!new RegExp(`(?:^|\\D)${mdbookVersion.replaceAll(".", "\\.")}(?:\\D|$)`).test(versionOutput)) {
    throw new Error(
      `Expected mdBook ${mdbookVersion}, received "${versionOutput || "unknown"}". ` +
        `Install it with: cargo install mdbook --version ${mdbookVersion} --locked`,
    );
  }
}
