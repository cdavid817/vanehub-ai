import type { UpdateChannel, UpdateManifestCandidate } from "../types/desktop-update";

interface SemVer { major: number; minor: number; patch: number; prerelease: Array<number | string>; }
const semverPattern = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

export function parseDesktopVersion(value: string): SemVer | null {
  const match = semverPattern.exec(value.trim());
  if (!match) return null;
  const prerelease = match[4]?.split(".").map((part) => (/^(0|[1-9]\d*)$/.test(part) ? Number(part) : part)) ?? [];
  return { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]), prerelease };
}

function comparePrerelease(left: SemVer["prerelease"], right: SemVer["prerelease"]) {
  if (left.length === 0 || right.length === 0) return left.length === right.length ? 0 : left.length === 0 ? 1 : -1;
  for (let index = 0; index < Math.max(left.length, right.length); index += 1) {
    const a = left[index]; const b = right[index];
    if (a === undefined || b === undefined) return a === undefined ? -1 : 1;
    if (a === b) continue;
    if (typeof a === "number" && typeof b === "string") return -1;
    if (typeof a === "string" && typeof b === "number") return 1;
    return a < b ? -1 : 1;
  }
  return 0;
}

export function compareDesktopVersions(left: string, right: string) {
  const a = parseDesktopVersion(left); const b = parseDesktopVersion(right);
  if (!a || !b) throw new Error("Invalid semantic version");
  for (const key of ["major", "minor", "patch"] as const) if (a[key] !== b[key]) return a[key] > b[key] ? 1 : -1;
  return comparePrerelease(a.prerelease, b.prerelease);
}

export function defaultUpdateChannel(version: string): UpdateChannel { return parseDesktopVersion(version)?.prerelease.length ? "preview" : "stable"; }
export function isUpdateAdmissible(current: string, candidate: string, channel: UpdateChannel) {
  const parsed = parseDesktopVersion(candidate);
  if (!parsed || (channel === "stable" && parsed.prerelease.length > 0)) return false;
  try { return compareDesktopVersions(candidate, current) > 0; } catch { return false; }
}
export function validateUpdateManifestCandidate(value: unknown): value is UpdateManifestCandidate {
  if (typeof value !== "object" || value === null) return false;
  const item = value as Record<string, unknown>;
  return parseDesktopVersion(String(item.version ?? "")) !== null && (item.channel === "stable" || item.channel === "preview")
    && typeof item.signature === "string" && item.signature.length >= 40 && typeof item.url === "string"
    && item.url.startsWith("https://") && (item.notes === undefined || typeof item.notes === "string");
}
export function evaluateUpdatePolicyBatch(cases: ReadonlyArray<readonly [string, string, UpdateChannel]>) {
  return cases.reduce((count, [current, candidate, channel]) => count + Number(isUpdateAdmissible(current, candidate, channel)), 0);
}
