import type { SkillScopeInput } from "../types/skill";

// Shared by the composition root's skill CRUD and by the extracted drift client: both must derive
// the same canonical form, so this stays one function rather than a copy per module.
export function normalizeWebPath(value: string, label: string) {
  const raw = value.trim().replaceAll("\\", "/");
  if (!raw) throw new Error(`${label} is required`);
  const drive = raw.match(/^[a-zA-Z]:/)?.[0];
  const absolute = raw.startsWith("/") || Boolean(drive);
  const remainder = drive ? raw.slice(drive.length) : raw;
  const segments: string[] = [];
  for (const segment of remainder.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (segments.length > 0 && segments.at(-1) !== "..") segments.pop();
      else if (!absolute) segments.push(segment);
      continue;
    }
    segments.push(segment);
  }
  const prefix = drive ? drive.toUpperCase() : raw.startsWith("/") ? "/" : "";
  const normalized = `${prefix}${prefix && prefix !== "/" && segments.length > 0 ? "/" : ""}${segments.join("/")}`;
  if (!normalized) throw new Error(`${label} is required`);
  return normalized;
}

export function normalizeWebSkillLocation(input: SkillScopeInput): SkillScopeInput {
  if (input.scope === "global") return { scope: "global", workspacePath: null };
  return { scope: "workspace", workspacePath: normalizeWebPath(input.workspacePath ?? "", "Workspace path") };
}
