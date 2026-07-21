export function normalizeDisplayPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) return `\\\\${path.slice(8)}`;
  if (path.startsWith("\\\\?\\")) return path.slice(4);
  if (path.startsWith("//?/UNC/")) return `//${path.slice(8)}`;
  if (path.startsWith("//?/")) return path.slice(4);
  return path;
}

export function folderNameFromPath(path: string): string {
  const normalized = normalizeDisplayPath(path).replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.at(-1) ?? "";
}

export function timestampForSessionName(date = new Date()): string {
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}-${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`;
}

export function defaultSessionTitleFromPath(path: string, date = new Date()): string {
  const folder = folderNameFromPath(path);
  return folder ? `${folder}-${timestampForSessionName(date)}` : "";
}
