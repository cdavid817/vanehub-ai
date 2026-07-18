import type { CliInstallSource, CliLifecycleEligibility, CliToolStatus } from "../../types/agent";

export type CliVersionAction = "install" | "upgrade" | "downgrade" | "current" | "manual" | "unavailable";

export function isManagedCliLifecycle(value: CliLifecycleEligibility) {
  return value === "npm" || value === "wget" || value === "winget";
}

function parseVersion(version: string) {
  const normalized = version.trim().replace(/^v/i, "");
  if (normalized.includes("-")) return null;
  const parts = normalized.split(".");
  if (parts.length === 0) return null;
  const numbers = parts.map((part) => {
    if (!/^\d+$/.test(part)) return null;
    return Number(part);
  });
  if (numbers.some((part) => part === null)) return null;
  return numbers as number[];
}

export function compareStableVersions(left: string, right: string) {
  const leftParts = parseVersion(left);
  const rightParts = parseVersion(right);
  if (!leftParts || !rightParts) return null;
  const maxLength = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < maxLength; index += 1) {
    const leftPart = leftParts[index] ?? 0;
    const rightPart = rightParts[index] ?? 0;
    if (leftPart > rightPart) return 1;
    if (leftPart < rightPart) return -1;
  }
  return 0;
}

export function deriveCliVersionAction(tool: CliToolStatus, targetVersion: string | null): CliVersionAction {
  if (tool.installed !== true) return targetVersion ? "install" : "unavailable";
  const activeInstallation = tool.installations.find((installation) => installation.isActive) ?? tool.installations[0];
  if (activeInstallation && !activeInstallation.runnable) return "manual";
  if (!tool.currentVersion && isManagedCliLifecycle(tool.lifecycleEligibility)) return "upgrade";
  if (!tool.currentVersion) return "unavailable";
  if (!targetVersion && isManagedCliLifecycle(tool.lifecycleEligibility)) return "upgrade";
  if (!targetVersion) return "unavailable";
  const comparison = compareStableVersions(targetVersion, tool.currentVersion);
  if (comparison === null) return isManagedCliLifecycle(tool.lifecycleEligibility) ? "upgrade" : "unavailable";
  if (comparison > 0) return "upgrade";
  if (comparison < 0) return tool.lifecycleEligibility === "npm" ? "downgrade" : "manual";
  return isManagedCliLifecycle(tool.lifecycleEligibility) ? "upgrade" : "current";
}

export function isBulkCliUpgradeEligible(tool: CliToolStatus) {
  if (!isManagedCliLifecycle(tool.lifecycleEligibility)) return false;
  if (tool.installed !== true || !tool.currentVersion || !tool.latestVersion) return false;
  if (tool.installations.length > 1) return false;
  return compareStableVersions(tool.latestVersion, tool.currentVersion) === 1;
}

export type CliLifecycleGuidance =
  | { kind: "broken"; key: "cli.guidance.checkEnv" }
  | { kind: "multiple"; key: "cli.guidance.multipleInstallations" }
  | { kind: "source-native"; key: "cli.guidance.sourceNative"; source: CliInstallSource }
  | { kind: "manual"; key: "cli.guidance.manual" };

export function deriveCliLifecycleGuidance(tool: CliToolStatus): CliLifecycleGuidance | null {
  if (tool.lifecycleEligibility !== "manual") return null;
  const activeInstallation = tool.installations.find((installation) => installation.isActive) ?? tool.installations[0];
  if (activeInstallation && !activeInstallation.runnable) return { kind: "broken", key: "cli.guidance.checkEnv" };
  if (tool.conflictState !== "none" || tool.installations.length > 1) return { kind: "multiple", key: "cli.guidance.multipleInstallations" };
  if (activeInstallation && activeInstallation.source !== "npm") {
    return { kind: "source-native", key: "cli.guidance.sourceNative", source: activeInstallation.source };
  }
  return { kind: "manual", key: "cli.guidance.manual" };
}
