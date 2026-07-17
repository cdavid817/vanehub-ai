import { appFontSizes, appLanguages, logLevels, type AppFontSize, type AppLanguage, type AppSettingKey, type AppSettings, type ClientLogEvent, type DetectedNetworkProxy, type LoggingPolicy, type NetworkProxyTestResult, type NodeInfo } from "../types/settings";
import { defaultThemeId, isUcdThemeId } from "../theme/theme-registry";

export interface SettingsService {
  getSettings(): Promise<AppSettings>;
  saveSetting(input: { key: AppSettingKey; value: AppSettings[AppSettingKey] }): Promise<AppSettings>;
  getNodeInfo(): Promise<NodeInfo>;
  openLogDirectory(): Promise<void>;
  testNetworkProxy(input: { url: string; bypass: string }): Promise<NetworkProxyTestResult>;
  scanNetworkProxies(): Promise<DetectedNetworkProxy[]>;
  reportClientLogEvent(event: ClientLogEvent): Promise<void>;
  subscribeSettingsEvents(handler: (event: SettingsStateEvent) => void): Promise<() => void>;
}

export interface SettingsStateEvent {
  kind: "settings-changed";
  key: AppSettingKey;
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
  networkProxyUrl: "",
  networkProxyBypass: "localhost,127.0.0.1,::1",
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

export function normalizeNetworkProxyBypass(value: string): string {
  return value
    .split(/[\s,]+/)
    .map((entry) => entry.trim())
    .filter(Boolean)
    .join(",");
}

function isNetworkProxyUrl(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return true;
  if (trimmed !== value || /[\u0000-\u001f\u007f]/.test(value)) return false;
  try {
    const parsed = new URL(trimmed);
    return ["http:", "https:", "socks5:", "socks5h:"].includes(parsed.protocol) && Boolean(parsed.hostname);
  } catch {
    return false;
  }
}

function isNetworkProxyBypass(value: string): boolean {
  return !/[\u0000-\u001f\u007f]/.test(value);
}

export function normalizeAppSettings(input: AppSettingsInput): AppSettings {
  const networkProxyBypass =
    typeof input.networkProxyBypass === "string" && isNetworkProxyBypass(input.networkProxyBypass)
      ? normalizeNetworkProxyBypass(input.networkProxyBypass)
      : defaultAppSettings.networkProxyBypass;
  return {
    applicationLanguage: isAppLanguage(input.applicationLanguage)
      ? input.applicationLanguage
      : defaultAppSettings.applicationLanguage,
    fontSize: isAppFontSize(input.fontSize) ? input.fontSize : defaultAppSettings.fontSize,
    theme: isUcdThemeId(input.theme) ? input.theme : defaultAppSettings.theme,
    defaultFolderPath:
      typeof input.defaultFolderPath === "string" ? input.defaultFolderPath : defaultAppSettings.defaultFolderPath,
    logDirectory: typeof input.logDirectory === "string" ? input.logDirectory : defaultAppSettings.logDirectory,
    networkProxyUrl:
      typeof input.networkProxyUrl === "string" && isNetworkProxyUrl(input.networkProxyUrl)
        ? input.networkProxyUrl
        : defaultAppSettings.networkProxyUrl,
    networkProxyBypass,
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
