import { chmod, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";

/**
 * A deterministic CLI environment for the native CLI Management layer.
 *
 * Everything the desktop runtime can reach during this layer is written here first: the CLIs it
 * discovers, the package manager it would mutate through, and the file each of them records its
 * arguments in. Nothing in it touches the host -- no real npm, no real WinGet, no vendor URL, no
 * credential store, and no PATH entry the machine already had.
 *
 * The layouts below are per platform because the thing under test is per platform. Windows
 * launcher families, Homebrew's two prefixes, and Linux's four install roots produce different
 * discovery answers, and a fixture that models only the host it happens to run on cannot show
 * that. The layout for a platform without a runner is still built and asserted by
 * `desktop:unit:test`; it is never reported as a passing platform.
 */

/** Recorded by every fake so a guard can prove which binary answered. */
export const FIXTURE_MARKER = "vanehub-cli-management-fixture";

/** Where each fake appends one JSON line per invocation. */
export const INVOCATION_LOG = "invocations.jsonl";

/** The versions the fixture starts at, before any mutation. */
export const INITIAL_VERSIONS = Object.freeze({
  claude: "1.2.0",
  codex: "0.40.0",
  gemini: "3.1.0",
  opencode: "0.9.0",
});

/** What a successful fake npm install moves `claude` to. */
export const UPGRADE_TARGET = "1.3.0";

/**
 * Directory roles, in the order they are placed on PATH.
 *
 * `shadowing` comes first on purpose: it holds a launcher that cannot run, so the host's PATH
 * reaches a broken binary before the healthy one. That is the case the conflict contract exists
 * for, and it is the case a fixture built from one healthy directory can never produce.
 */
export const LAYOUTS = Object.freeze({
  win32: {
    /** `PATHEXT` decides which of `claude`, `claude.cmd`, `claude.ps1` a shell reaches. */
    pathext: [".COM", ".EXE", ".BAT", ".CMD", ".PS1"],
    directories: [
      { role: "shadowing", segments: ["nvm", "v18.0.0"], note: "an NVM version directory earlier on PATH" },
      { role: "npm-global", segments: ["npm", "node_modules", ".bin"] },
      { role: "winget-links", segments: ["winget", "Links"] },
      { role: "user-path", segments: ["Users", "fixture", "bin"] },
    ],
    // One npm global install writes three launchers side by side; grouping them into one logical
    // installation is what stops the page reporting three competing copies of the same thing.
    launcherFamily: ["", ".cmd", ".ps1"],
    packageManager: { name: "npm", extension: ".cmd" },
    secondaryPackageManager: { name: "winget", extension: ".cmd" },
  },
  darwin: {
    pathext: [],
    directories: [
      { role: "shadowing", segments: ["usr", "local", "bin"], note: "Intel Homebrew prefix, ahead of the arm64 one" },
      { role: "npm-global", segments: ["opt", "homebrew", "bin"], note: "arm64 Homebrew prefix" },
      { role: "cellar", segments: ["opt", "homebrew", "Cellar", "fixture", "1.2.0", "bin"] },
      { role: "user-path", segments: ["Users", "fixture", ".local", "bin"] },
    ],
    launcherFamily: [""],
    packageManager: { name: "npm", extension: "" },
    secondaryPackageManager: null,
  },
  linux: {
    pathext: [],
    directories: [
      { role: "shadowing", segments: ["usr", "bin"] },
      { role: "npm-global", segments: ["home", "fixture", ".npm-global", "bin"] },
      { role: "version-manager", segments: ["home", "fixture", ".nvm", "versions", "node", "v18.0.0", "bin"] },
      { role: "user-path", segments: ["home", "fixture", ".local", "bin"] },
    ],
    launcherFamily: [""],
    packageManager: { name: "npm", extension: "" },
    secondaryPackageManager: null,
  },
});

export function layoutFor(platform = process.platform) {
  return LAYOUTS[platform === "win32" ? "win32" : platform === "darwin" ? "darwin" : "linux"];
}

/** A Node script that answers a CLI's probes and records every call. */
function cliScript({ name, versionFile, logPath }) {
  return `#!/usr/bin/env node
// A managed CLI, as far as the desktop runtime can tell. It answers the probes the tool registry
// declares and records every invocation, so a guard can prove no real binary answered instead.
const fs = require("node:fs");
const LOG = ${JSON.stringify(logPath)};
const VERSION_FILE = ${JSON.stringify(versionFile)};
const args = process.argv.slice(2);
fs.appendFileSync(LOG, JSON.stringify({
  marker: ${JSON.stringify(FIXTURE_MARKER)},
  tool: ${JSON.stringify(name)},
  argv: args,
  executable: __filename,
}) + "\\n");
function version() {
  try { return fs.readFileSync(VERSION_FILE, "utf8").trim(); } catch { return "0.0.0"; }
}
if (args[0] === "--version") { process.stdout.write(version() + "\\n"); process.exit(0); }
if (args[0] === "doctor") { process.stdout.write("All checks passed\\n"); process.exit(0); }
if (args[0] === "login" && args[1] === "status") { process.stdout.write("Logged in\\n"); process.exit(0); }
if (args[0] === "auth" && args[1] === "list") { process.stdout.write("anthropic  api key\\n"); process.exit(0); }
process.stdout.write("unsupported\\n");
process.exit(1);
`;
}

/** A launcher that exists on PATH and cannot run. Discovery must find it and call it broken. */
function brokenScript({ name, logPath }) {
  return `#!/usr/bin/env node
// Deliberately fails every invocation. It sits ahead of the healthy copy on PATH, which is what a
// stale shim from a removed installation looks like to a shell.
const fs = require("node:fs");
fs.appendFileSync(${JSON.stringify(logPath)}, JSON.stringify({
  marker: ${JSON.stringify(FIXTURE_MARKER)},
  tool: ${JSON.stringify(name)},
  argv: process.argv.slice(2),
  executable: __filename,
  broken: true,
}) + "\\n");
process.stderr.write("fixture: this launcher points at a target that no longer exists\\n");
process.exit(127);
`;
}

/**
 * A package manager that mutates the fixture instead of the machine.
 *
 * `install --global <package>@<version>` rewrites the version file the fake CLI reports, which is
 * what makes post-mutation verification a real observation rather than an assumed one.
 */
function packageManagerScript({ name, logPath, versionFiles }) {
  return `#!/usr/bin/env node
// Stands in for ${name}. It never reaches a registry, a network, or a credential store.
const fs = require("node:fs");
const args = process.argv.slice(2);
fs.appendFileSync(${JSON.stringify(logPath)}, JSON.stringify({
  marker: ${JSON.stringify(FIXTURE_MARKER)},
  tool: ${JSON.stringify(name)},
  argv: args,
  executable: __filename,
}) + "\\n");
const VERSION_FILES = ${JSON.stringify(versionFiles)};
if (args[0] === "view" || args[0] === "show") {
  // A source-native catalog answer. Shaped like npm's --json output for \`versions\`.
  process.stdout.write(JSON.stringify(["1.1.0", "1.2.0", "1.3.0"]) + "\\n");
  process.exit(0);
}
if (args[0] === "install" || args[0] === "upgrade") {
  const spec = args.find((value) => value.includes("@") && !value.startsWith("-")) ?? "";
  const at = spec.lastIndexOf("@");
  const version = at > 0 ? spec.slice(at + 1) : "";
  const target = Object.entries(VERSION_FILES).find(([pkg]) => spec.startsWith(pkg));
  if (!target || !version) { process.stderr.write("fixture: nothing to install\\n"); process.exit(1); }
  if (version === "9.9.9-fails") { process.stderr.write("fixture: install refused\\n"); process.exit(1); }
  fs.writeFileSync(target[1], version + "\\n");
  process.stdout.write("added 1 package\\n");
  process.exit(0);
}
process.stderr.write("fixture: unsupported command\\n");
process.exit(1);
`;
}

/** Writes one executable script, with the launcher wrapper each platform needs. */
async function writeExecutable(directory, base, extension, body) {
  const target = path.join(directory, `${base}${extension}`);
  if (extension === ".cmd") {
    // A `.cmd` shim is what npm writes on Windows, and it is what `PATHEXT` reaches first.
    await writeFile(target, `@echo off\r\nnode "%~dp0${base}" %*\r\n`, "utf8");
    return target;
  }
  if (extension === ".ps1") {
    await writeFile(target, `#!/usr/bin/env pwsh\nnode "$PSScriptRoot/${base}" @args\n`, "utf8");
    return target;
  }
  await writeFile(target, body, "utf8");
  await chmod(target, 0o755);
  return target;
}

/**
 * Builds the fixture tree and returns what the layer needs to drive and to check it.
 *
 * `root` is created under the OS temp directory and removed by `disposeCliManagementFixture`, so
 * nothing is written inside the repository or the user's profile.
 */
export async function createCliManagementFixture({ root, platform = process.platform } = {}) {
  const layout = layoutFor(platform);
  const fixtureRoot = root ?? path.join(os.tmpdir(), `vanehub-cli-management-${process.pid}`);
  await rm(fixtureRoot, { recursive: true, force: true });
  const logPath = path.join(fixtureRoot, INVOCATION_LOG);
  const versionsDir = path.join(fixtureRoot, "versions");
  await mkdir(versionsDir, { recursive: true });
  await writeFile(logPath, "", "utf8");

  const directories = {};
  for (const entry of layout.directories) {
    const directory = path.join(fixtureRoot, ...entry.segments);
    await mkdir(directory, { recursive: true });
    directories[entry.role] = directory;
  }

  const versionFiles = {};
  for (const [tool, version] of Object.entries(INITIAL_VERSIONS)) {
    const file = path.join(versionsDir, `${tool}.txt`);
    await writeFile(file, `${version}\n`, "utf8");
    versionFiles[tool] = file;
  }

  // The healthy copies, in the npm global directory, with the full launcher family so one install
  // is one installation rather than three.
  for (const tool of Object.keys(INITIAL_VERSIONS)) {
    const body = cliScript({ name: tool, versionFile: versionFiles[tool], logPath });
    for (const extension of layout.launcherFamily) {
      await writeExecutable(directories["npm-global"], tool, extension, body);
    }
  }

  // One broken launcher ahead of the healthy one, so PATH reaches something that cannot run.
  await writeExecutable(
    directories.shadowing,
    "codex",
    layout.launcherFamily[0],
    brokenScript({ name: "codex", logPath }),
  );
  if (layout.launcherFamily.includes(".cmd")) {
    await writeExecutable(directories.shadowing, "codex", ".cmd", "");
  }

  const packageManagerVersionFiles = {
    "@anthropic-ai/claude-code": versionFiles.claude,
    "@openai/codex": versionFiles.codex,
    "@google/gemini-cli": versionFiles.gemini,
    "opencode-ai": versionFiles.opencode,
  };
  const managerBody = packageManagerScript({
    name: layout.packageManager.name,
    logPath,
    versionFiles: packageManagerVersionFiles,
  });
  await writeExecutable(directories["npm-global"], layout.packageManager.name, "", managerBody);
  if (layout.packageManager.extension) {
    await writeExecutable(
      directories["npm-global"],
      layout.packageManager.name,
      layout.packageManager.extension,
      managerBody,
    );
  }
  if (layout.secondaryPackageManager) {
    const secondary = packageManagerScript({
      name: layout.secondaryPackageManager.name,
      logPath,
      versionFiles: packageManagerVersionFiles,
    });
    await writeExecutable(directories["winget-links"], layout.secondaryPackageManager.name, "", secondary);
    await writeExecutable(
      directories["winget-links"],
      layout.secondaryPackageManager.name,
      layout.secondaryPackageManager.extension,
      secondary,
    );
  }

  const pathEntries = layout.directories.map((entry) => directories[entry.role]);
  return {
    root: fixtureRoot,
    directories,
    versionFiles,
    logPath,
    pathEntries,
    /** Prepended, never replacing: the runtime still needs `node` to run the fakes themselves. */
    pathValue: [...pathEntries, process.env.PATH ?? ""].join(path.delimiter),
    pathext: layout.pathext.join(";"),
  };
}

export async function disposeCliManagementFixture(fixture) {
  await rm(fixture.root, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}
