import type { CliToolStatus } from "../../types/agent";

export type CliVersionAction = "install" | "upgrade" | "downgrade" | "current" | "manual" | "unavailable";

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
  if (tool.lifecycleEligibility === "manual") return "manual";
  if (tool.lifecycleEligibility !== "npm") return "unavailable";
  if (!targetVersion) return "unavailable";
  if (tool.installed !== true || !tool.currentVersion) return "install";
  const comparison = compareStableVersions(targetVersion, tool.currentVersion);
  if (comparison === null) return "unavailable";
  if (comparison > 0) return "upgrade";
  if (comparison < 0) return "downgrade";
  return "current";
}
