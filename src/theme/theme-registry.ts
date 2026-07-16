export const themeStorageKey = "vanehub.uiStyle";

export const ucdThemes = [
  { id: "futuristic", label: "Futuristic", displayName: "Futuristic" },
  { id: "minimal", label: "Minimal", displayName: "Minimal" },
] as const;

export type UcdThemeId = (typeof ucdThemes)[number]["id"];

export const defaultThemeId: UcdThemeId = "futuristic";

export function isUcdThemeId(value: unknown): value is UcdThemeId {
  return typeof value === "string" && ucdThemes.some((theme) => theme.id === value);
}

export function normalizeThemeId(value: unknown): UcdThemeId {
  return isUcdThemeId(value) ? value : defaultThemeId;
}

export function getThemeDefinition(themeId: UcdThemeId) {
  return ucdThemes.find((theme) => theme.id === themeId) ?? ucdThemes[0];
}

export function getNextThemeId(themeId: UcdThemeId): UcdThemeId {
  const currentIndex = ucdThemes.findIndex((theme) => theme.id === themeId);
  const nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % ucdThemes.length;
  return ucdThemes[nextIndex].id;
}
