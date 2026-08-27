export type PythonDiscoveryAvailability = "available" | "unavailable";
export type PythonDiscoverySource = "configured" | "path" | "windows_launcher";
export type PythonCompatibility = "compatible" | "unsupported";
export type PythonDiscoveryReason =
  | "manual_configuration_required"
  | "native_unavailable"
  | "unsupported_version";

export interface PythonVersion {
  major: number;
  minor: number;
  patch: number;
}

export interface PythonEnvironmentCandidate {
  executablePath: string;
  version: PythonVersion;
  compatibility: PythonCompatibility;
  reasonCode: PythonDiscoveryReason | null;
  source: PythonDiscoverySource;
}

export interface PythonEnvironmentDiscovery {
  availability: PythonDiscoveryAvailability;
  reasonCode: PythonDiscoveryReason | null;
  candidates: PythonEnvironmentCandidate[];
}

const SOURCES = new Set<PythonDiscoverySource>(["configured", "path", "windows_launcher"]);
const COMPATIBILITY = new Set<PythonCompatibility>(["compatible", "unsupported"]);
const REASONS = new Set<PythonDiscoveryReason>([
  "manual_configuration_required",
  "native_unavailable",
  "unsupported_version",
]);

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function integer(value: unknown): number | null {
  return Number.isInteger(value) && Number(value) >= 0 ? Number(value) : null;
}

function reason(value: unknown): PythonDiscoveryReason | null | undefined {
  if (value === null) return null;
  return typeof value === "string" && REASONS.has(value as PythonDiscoveryReason)
    ? (value as PythonDiscoveryReason)
    : undefined;
}

function candidate(value: unknown): PythonEnvironmentCandidate | null {
  const item = record(value);
  const version = record(item?.version);
  if (!item || !version) return null;
  const major = integer(version.major);
  const minor = integer(version.minor);
  const patch = integer(version.patch);
  const parsedReason = reason(item.reasonCode);
  if (
    typeof item.executablePath !== "string" ||
    item.executablePath.trim().length === 0 ||
    major === null || minor === null || patch === null ||
    typeof item.compatibility !== "string" ||
    !COMPATIBILITY.has(item.compatibility as PythonCompatibility) ||
    typeof item.source !== "string" ||
    !SOURCES.has(item.source as PythonDiscoverySource) ||
    parsedReason === undefined
  ) return null;
  return {
    executablePath: item.executablePath,
    version: { major, minor, patch },
    compatibility: item.compatibility as PythonCompatibility,
    reasonCode: parsedReason,
    source: item.source as PythonDiscoverySource,
  };
}

export function normalizePythonEnvironmentDiscovery(value: unknown): PythonEnvironmentDiscovery {
  const input = record(value);
  const parsedReason = reason(input?.reasonCode);
  if (
    !input ||
    (input.availability !== "available" && input.availability !== "unavailable") ||
    !Array.isArray(input.candidates) ||
    parsedReason === undefined
  ) throw new Error("LOCAL_MEDIA_DISCOVERY_INVALID_RESPONSE");
  const candidates = input.candidates.map(candidate);
  if (candidates.some((item) => item === null)) {
    throw new Error("LOCAL_MEDIA_DISCOVERY_INVALID_RESPONSE");
  }
  return {
    availability: input.availability,
    reasonCode: parsedReason,
    candidates: candidates as PythonEnvironmentCandidate[],
  };
}
