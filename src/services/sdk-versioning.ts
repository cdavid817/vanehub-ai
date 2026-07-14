import type { SdkVersionAction } from "../types/sdk";

export function normalizeSdkVersion(version?: string | null): string | undefined {
  const trimmed = version?.trim();
  if (!trimmed) return undefined;
  return trimmed.startsWith("v") || trimmed.startsWith("V") ? trimmed.slice(1) : trimmed;
}

export function compareSdkVersions(left?: string | null, right?: string | null): number {
  const normalizedLeft = normalizeSdkVersion(left);
  const normalizedRight = normalizeSdkVersion(right);
  if (!normalizedLeft || !normalizedRight) return 0;

  const leftParts = normalizedLeft.split(/[.-]/);
  const rightParts = normalizedRight.split(/[.-]/);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const leftValue = Number.parseInt(leftParts[index] ?? "0", 10);
    const rightValue = Number.parseInt(rightParts[index] ?? "0", 10);
    if (Number.isNaN(leftValue) || Number.isNaN(rightValue)) continue;
    if (leftValue !== rightValue) return leftValue - rightValue;
  }

  return 0;
}

export function getSdkVersionAction({
  installed,
  installedVersion,
  requestedVersion,
}: {
  installed: boolean;
  installedVersion?: string | null;
  requestedVersion?: string | null;
}): SdkVersionAction {
  if (!installed) return "install";
  const comparison = compareSdkVersions(installedVersion, requestedVersion);
  if (comparison === 0) return "current";
  return comparison < 0 ? "update" : "rollback";
}

export function buildSdkVersionOptions({
  availableVersions = [],
  fallbackVersions = [],
  installedVersion,
}: {
  availableVersions?: string[];
  fallbackVersions?: string[];
  installedVersion?: string | null;
}) {
  const seen = new Set<string>();
  return [...availableVersions, ...fallbackVersions, installedVersion].reduce<string[]>((versions, version) => {
    const normalized = normalizeSdkVersion(version);
    if (!normalized || seen.has(normalized)) return versions;
    seen.add(normalized);
    versions.push(normalized);
    return versions;
  }, []);
}
