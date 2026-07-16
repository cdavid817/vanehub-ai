import { appFontSizes, appLanguages, logLevels, type AppFontSize, type AppLanguage, type AppSettingKey, type AppSettings, type ClientLogEvent, type LoggingPolicy, type NodeInfo } from "../types/settings";
import { defaultThemeId, isUcdThemeId } from "../theme/theme-registry";

export interface SettingsService {
  getSettings(): Promise<AppSettings>;
  saveSetting(input: { key: AppSettingKey; value: AppSettings[AppSettingKey] }): Promise<AppSettings>;
  getNodeInfo(): Promise<NodeInfo>;
  openLogDirectory(): Promise<void>;
  reportClientLogEvent(event: ClientLogEvent): Promise<void>;
}

export const defaultLoggingPolicy: LoggingPolicy = {
  retentionDays: 30,
  archiveEnabled: true,
  redactionEnabled: true,
  levels: [...logLevels],
  canOpenDirectory: false,
};

export const defaultAppSettings: AppSettings = {
  applicationLanguage: "zh-CN",
  fontSize: "14px",
  theme: defaultThemeId,
  defaultFolderPath: "",
  logDirectory: "",
  loggingPolicy: defaultLoggingPolicy,
};

export function isAppLanguage(value: unknown): value is AppLanguage {
  return typeof value === "string" && appLanguages.includes(value as AppLanguage);
}

export function isAppFontSize(value: unknown): value is AppFontSize {
  return typeof value === "string" && appFontSizes.includes(value as AppFontSize);
}

function normalizeLoggingPolicy(input: unknown): LoggingPolicy {
  if (!input || typeof input !== "object") return defaultLoggingPolicy;
  const value = input as Partial<LoggingPolicy>;
  const levels = Array.isArray(value.levels)
    ? value.levels.filter((level): level is LoggingPolicy["levels"][number] => logLevels.includes(level as LoggingPolicy["levels"][number]))
    : defaultLoggingPolicy.levels;

  return {
    retentionDays: typeof value.retentionDays === "number" ? value.retentionDays : defaultLoggingPolicy.retentionDays,
    archiveEnabled: typeof value.archiveEnabled === "boolean" ? value.archiveEnabled : defaultLoggingPolicy.archiveEnabled,
    redactionEnabled: typeof value.redactionEnabled === "boolean" ? value.redactionEnabled : defaultLoggingPolicy.redactionEnabled,
    levels: levels.length ? levels : defaultLoggingPolicy.levels,
    canOpenDirectory: typeof value.canOpenDirectory === "boolean" ? value.canOpenDirectory : defaultLoggingPolicy.canOpenDirectory,
  };
}

type AppSettingsInput = Partial<Record<AppSettingKey | "loggingPolicy", unknown>>;

export function normalizeAppSettings(input: AppSettingsInput): AppSettings {
  return {
    applicationLanguage: isAppLanguage(input.applicationLanguage)
      ? input.applicationLanguage
      : defaultAppSettings.applicationLanguage,
    fontSize: isAppFontSize(input.fontSize) ? input.fontSize : defaultAppSettings.fontSize,
    theme: isUcdThemeId(input.theme) ? input.theme : defaultAppSettings.theme,
    defaultFolderPath:
      typeof input.defaultFolderPath === "string" ? input.defaultFolderPath : defaultAppSettings.defaultFolderPath,
    logDirectory: typeof input.logDirectory === "string" ? input.logDirectory : defaultAppSettings.logDirectory,
    loggingPolicy: normalizeLoggingPolicy(input.loggingPolicy),
  };
}

export function validateSettingValue<K extends AppSettingKey>(key: K, value: AppSettings[K]): AppSettings[K] {
  const normalized = normalizeAppSettings({ [key]: value });
  if (normalized[key] !== value) {
    throw new Error(`Invalid setting value for ${key}.`);
  }
  return value;
}
