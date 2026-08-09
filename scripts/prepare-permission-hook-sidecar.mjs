import console from "node:console";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, isAbsolute, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { execFileSync, spawnSync } from "node:child_process";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const binaryName = "vanehub-permission-hook";

export function parseHostTriple(rustcVerboseVersion) {
  const match = rustcVerboseVersion.match(/^host:\s*(\S+)$/m);
  if (!match) throw new Error("Could not determine the host target from rustc -vV.");
  return match[1];
}

export function resolvePreparationOptions(args, rustcVerboseVersion) {
  const release = args.includes("--release");
  const targetArgument = args.find((argument) => argument.startsWith("--target="));
  const target = targetArgument?.slice("--target=".length)
    || parseHostTriple(rustcVerboseVersion);
  if (!/^[A-Za-z0-9_.-]+$/.test(target)) {
    throw new Error(`Invalid Rust target triple: ${target}`);
  }
  return { profile: release ? "release" : "debug", release, target };
}

export function sidecarPaths(root, target, profile, targetDirectory = undefined) {
  const extension = target.includes("windows") ? ".exe" : "";
  const cargoTarget = targetDirectory
    ? (isAbsolute(targetDirectory) ? targetDirectory : resolve(root, targetDirectory))
    : resolve(root, "src-tauri", "target");
  return {
    source: resolve(cargoTarget, target, profile, `${binaryName}${extension}`),
    staged: resolve(root, "src-tauri", "binaries", `${binaryName}-${target}${extension}`),
  };
}

export function preparePermissionHookSidecar({
  args = process.argv.slice(2),
  env = process.env,
  root = repositoryRoot,
} = {}) {
  const rustcVerboseVersion = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const options = resolvePreparationOptions(args, rustcVerboseVersion);
  const cargoArgs = [
    "build",
    "--manifest-path",
    resolve(root, "src-tauri", "Cargo.toml"),
    "--bin",
    binaryName,
    "--target",
    options.target,
  ];
  if (options.release) cargoArgs.push("--release");

  const result = spawnSync("cargo", cargoArgs, { cwd: root, env, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`Permission-hook sidecar build failed with status ${String(result.status)}.`);
  }

  const paths = sidecarPaths(root, options.target, options.profile, env.CARGO_TARGET_DIR);
  if (!existsSync(paths.source)) {
    throw new Error(`Permission-hook sidecar was not produced at ${paths.source}.`);
  }
  mkdirSync(dirname(paths.staged), { recursive: true });
  copyFileSync(paths.source, paths.staged);
  console.log(`Prepared permission-hook sidecar for ${options.target}: ${paths.staged}`);
  return { ...options, ...paths };
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    preparePermissionHookSidecar();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
