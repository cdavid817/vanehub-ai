import { spawnSync } from "node:child_process";
import { statSync } from "node:fs";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { DesktopVerificationError } from "./verification-error.mjs";

export async function loadDesktopMetadata(repoRoot, command = spawnSync) {
  const manifestPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
  const result = command("cargo", ["metadata", "--no-deps", "--format-version", "1", "--manifest-path", manifestPath], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.error || result.status !== 0) {
    throw new DesktopVerificationError("FAILED", "Cargo metadata could not be loaded.", {
      stderr: String(result.stderr ?? result.error?.message ?? ""),
    });
  }
  const cargo = JSON.parse(result.stdout);
  const packageInfo = cargo.packages.find((candidate) => candidate.manifest_path === manifestPath)
    ?? cargo.packages.find((candidate) => candidate.name === "vanehub-ai");
  if (!packageInfo) throw new DesktopVerificationError("FAILED", "The VaneHub AI Cargo package was not found.");
  const tauri = JSON.parse(await readFile(path.join(repoRoot, "src-tauri", "tauri.conf.json"), "utf8"));
  return {
    binaryName: packageInfo.default_run ?? packageInfo.name,
    packageName: packageInfo.name,
    productName: tauri.productName,
    targetDirectory: path.resolve(cargo.target_directory),
  };
}

export function resolveDesktopArtifact({
  metadata,
  host,
  profile = "debug",
  buildStartedAt,
  stat = statSync,
}) {
  const executablePath = path.join(
    metadata.targetDirectory,
    host.targetTriple,
    profile,
    `${metadata.binaryName}${host.extension}`,
  );
  let artifactStat;
  try {
    artifactStat = stat(executablePath);
  } catch {
    throw new DesktopVerificationError("FAILED", "The requested desktop artifact was not produced.", {
      inspectedPaths: [executablePath],
    });
  }
  if (!artifactStat.isFile()) {
    throw new DesktopVerificationError("FAILED", "The resolved desktop artifact is not a file.", {
      inspectedPaths: [executablePath],
    });
  }
  if (buildStartedAt && artifactStat.mtimeMs + 1_000 < buildStartedAt) {
    throw new DesktopVerificationError("FAILED", "The resolved desktop artifact predates the current build.", {
      executablePath,
      artifactModifiedAt: artifactStat.mtimeMs,
      buildStartedAt,
    });
  }
  return {
    platform: host.platform,
    architecture: host.architecture,
    targetTriple: host.targetTriple,
    profile,
    executablePath,
    productName: metadata.productName,
    testBuild: true,
  };
}
