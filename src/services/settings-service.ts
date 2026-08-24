import { isSupportedAppLanguage } from "../i18n/supported-locales";
import { appFontSizes, contextQualityRetentionDaysOptions, countCustomInstructionsCharacters, customInstructionsFieldCharacterLimit, logLevels, type AppFontSize, type AppLanguage, type AppSettingKey, type AppSettings, type ClientLogEvent, type ContextQualityRetentionDays, type DataManagementInfo, type DetectedNetworkProxy, type LoggingPolicy, type NetworkProxyTestResult, type NodeInfo } from "../types/settings";
import { policyTemplateNames, type PolicyTemplateName } from "../types/permissions";
import { defaultThemeId, isUcdThemeId } from "../theme/theme-registry";

export interface SettingsService {
  getSettings(): Promise<AppSettings>;
  saveSetting(input: { key: AppSettingKey; value: AppSettings[AppSettingKey]; expectedPersonalizationRevision?: number }): Promise<AppSettings>;
  setLaunchOnStartup(enabled: boolean): Promise<AppSettings>;
  getNodeInfo(): Promise<NodeInfo>;
  getDataManagementInfo(): Promise<DataManagementInfo>;
  openDatabaseDirectory(): Promise<void>;
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
  launchOnStartup: false,
  defaultPolicyTemplate: "standard",
  loggingPolicy: defaultLoggingPolicy,
  customInstructionsAboutUser: "",
  customInstructionsStyleRules: "",
  customInstructionsEnabled: true,
  memoryEnabled: true,
  memoryToolAssistedChatsEnabled: true,
  automaticContextCompactionEnabled: true,
  contextQualityRetentionDays: 30,
  personalizationRevision: 0,
};

export function isAppLanguage(value: unknown): value is AppLanguage {
  return isSupportedAppLanguage(value);
}

export function isAppFontSize(value: unknown): value is AppFontSize {
  return typeof value === "string" && appFontSizes.includes(value as AppFontSize);
}

export function isPolicyTemplateName(value: unknown): value is PolicyTemplateName {
  return typeof value === "string" && policyTemplateNames.includes(value as PolicyTemplateName);
}

export function isContextQualityRetentionDays(value: unknown): value is ContextQualityRetentionDays {
  return typeof value === "number"
    && contextQualityRetentionDaysOptions.includes(value as ContextQualityRetentionDays);
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

// `personalizationRevision` is read-only: the native side reports it and no caller may set it, so
// it is not an `AppSettingKey` and is admitted here separately.
type AppSettingsInput = Partial<Record<AppSettingKey | "loggingPolicy" | "personalizationRevision", unknown>>;

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

function isValidCustomInstructionsField(value: unknown): value is string {
  return typeof value === "string" && countCustomInstructionsCharacters(value) <= customInstructionsFieldCharacterLimit;
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
    launchOnStartup:
      typeof input.launchOnStartup === "boolean" ? input.launchOnStartup : defaultAppSettings.launchOnStartup,
    defaultPolicyTemplate: isPolicyTemplateName(input.defaultPolicyTemplate)
      ? input.defaultPolicyTemplate
      : defaultAppSettings.defaultPolicyTemplate,
    loggingPolicy: normalizeLoggingPolicy(input.loggingPolicy),
    customInstructionsAboutUser: isValidCustomInstructionsField(input.customInstructionsAboutUser)
      ? input.customInstructionsAboutUser
      : defaultAppSettings.customInstructionsAboutUser,
    customInstructionsStyleRules: isValidCustomInstructionsField(input.customInstructionsStyleRules)
      ? input.customInstructionsStyleRules
      : defaultAppSettings.customInstructionsStyleRules,
    customInstructionsEnabled:
      typeof input.customInstructionsEnabled === "boolean"
        ? input.customInstructionsEnabled
        : defaultAppSettings.customInstructionsEnabled,
    memoryEnabled:
      typeof input.memoryEnabled === "boolean" ? input.memoryEnabled : defaultAppSettings.memoryEnabled,
    memoryToolAssistedChatsEnabled:
      typeof input.memoryToolAssistedChatsEnabled === "boolean"
        ? input.memoryToolAssistedChatsEnabled
        : defaultAppSettings.memoryToolAssistedChatsEnabled,
    automaticContextCompactionEnabled:
      typeof input.automaticContextCompactionEnabled === "boolean"
        ? input.automaticContextCompactionEnabled
        : defaultAppSettings.automaticContextCompactionEnabled,
    contextQualityRetentionDays: isContextQualityRetentionDays(input.contextQualityRetentionDays)
      ? input.contextQualityRetentionDays
      : defaultAppSettings.contextQualityRetentionDays,
    personalizationRevision:
      typeof input.personalizationRevision === "number" && Number.isInteger(input.personalizationRevision) && input.personalizationRevision >= 0
        ? input.personalizationRevision
        : defaultAppSettings.personalizationRevision,
  };
}

export function validateSettingValue<K extends AppSettingKey>(key: K, value: AppSettings[K]): AppSettings[K] {
  const normalized = normalizeAppSettings({ [key]: value });
  if (normalized[key] !== value) {
    throw new Error(`Invalid setting value for ${key}.`);
  }
  return value;
}
