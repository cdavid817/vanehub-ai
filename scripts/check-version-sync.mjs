import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import console from "node:console";
import process from "node:process";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function readCargoPackageVersion(root) {
  const cargoToml = readFileSync(resolve(root, "src-tauri", "Cargo.toml"), "utf8");
  const cargoPackage = cargoToml.match(/\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!cargoPackage) {
    throw new Error("Could not read the [package] version from src-tauri/Cargo.toml");
  }
  return cargoPackage[1];
}

// GitHub sets GITHUB_REF_NAME for every ref, so falling back to it unconditionally made a
// workflow_dispatch run compare a branch name against `v<version>` and fail. That left the
// release workflow with no way to rehearse a build without first pushing a tag.
export function resolveReleaseTag(argv = process.argv, env = process.env) {
  const explicit = argv[2];
  if (explicit) return explicit;
  return env.GITHUB_REF_TYPE === "tag" ? env.GITHUB_REF_NAME : undefined;
}

export function checkVersionSync(root = repositoryRoot, tag = undefined) {
  const packageJson = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
  const tauriConfig = JSON.parse(
    readFileSync(resolve(root, "src-tauri", "tauri.conf.json"), "utf8"),
  );

  const versions = new Map([
    ["package.json", packageJson.version],
    ["src-tauri/Cargo.toml", readCargoPackageVersion(root)],
    ["src-tauri/tauri.conf.json", tauriConfig.version],
  ]);

  if (new Set(versions.values()).size !== 1) {
    const details = [...versions]
      .map(([file, version]) => `${file}: ${String(version)}`)
      .join("\n");
    throw new Error(`Project versions are not synchronized:\n${details}`);
  }

  const version = packageJson.version;
  // A pre-release identifier is part of the version, so the tag must carry it verbatim
  // rather than being normalized away.
  if (tag && tag !== `v${version}`) {
    throw new Error(`Release tag ${tag} does not match synchronized version v${version}`);
  }
  return version;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const version = checkVersionSync(repositoryRoot, resolveReleaseTag());
    console.log(`Version ${version} is synchronized across npm, Cargo, and Tauri.`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
