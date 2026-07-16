import { appFontSizes, appLanguages, type AppFontSize, type AppLanguage, type AppSettingKey, type AppSettings, type NodeInfo } from "../types/settings";
import { defaultThemeId, isUcdThemeId } from "../theme/theme-registry";

export interface SettingsService {
  getSettings(): Promise<AppSettings>;
  saveSetting(input: { key: AppSettingKey; value: AppSettings[AppSettingKey] }): Promise<AppSettings>;
  getNodeInfo(): Promise<NodeInfo>;
}

export const defaultAppSettings: AppSettings = {
  applicationLanguage: "zh-CN",
  fontSize: "14px",
  theme: defaultThemeId,
  defaultFolderPath: "",
};

export function isAppLanguage(value: unknown): value is AppLanguage {
  return typeof value === "string" && appLanguages.includes(value as AppLanguage);
}

export function isAppFontSize(value: unknown): value is AppFontSize {
  return typeof value === "string" && appFontSizes.includes(value as AppFontSize);
}

export function normalizeAppSettings(input: Partial<Record<AppSettingKey, unknown>>): AppSettings {
  return {
    applicationLanguage: isAppLanguage(input.applicationLanguage)
      ? input.applicationLanguage
      : defaultAppSettings.applicationLanguage,
    fontSize: isAppFontSize(input.fontSize) ? input.fontSize : defaultAppSettings.fontSize,
    theme: isUcdThemeId(input.theme) ? input.theme : defaultAppSettings.theme,
    defaultFolderPath:
      typeof input.defaultFolderPath === "string" ? input.defaultFolderPath : defaultAppSettings.defaultFolderPath,
  };
}

export function validateSettingValue<K extends AppSettingKey>(key: K, value: AppSettings[K]): AppSettings[K] {
  const normalized = normalizeAppSettings({ [key]: value });
  if (normalized[key] !== value) {
    throw new Error(`Invalid setting value for ${key}.`);
  }
  return value;
}
