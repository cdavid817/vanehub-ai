import type { UcdThemeId } from "../theme/theme-registry";
import { appLanguages, type AppLanguage } from "../i18n/supported-locales";
import type { PolicyTemplateName } from "./permissions";

export { appLanguages };
export type { AppLanguage };

export const appFontSizes = ["12px", "14px", "16px", "18px"] as const;
export type AppFontSize = (typeof appFontSizes)[number];

export const logLevels = ["error", "warn", "info", "debug"] as const;
export type LogLevel = (typeof logLevels)[number];

export type ClientLogEventKind = "error-boundary" | "critical-operation-failure";

export interface NetworkProxyTestResult {
  success: boolean;
  latencyMs: number;
  error: string | null;
}

export interface DetectedNetworkProxy {
  url: string;
  proxyType: string;
  port: number;
}

export interface LoggingPolicy {
  retentionDays: number;
  archiveEnabled: boolean;
  redactionEnabled: boolean;
  levels: LogLevel[];
  canOpenDirectory: boolean;
}

export interface DataManagementInfo {
  databasePath: string;
  databaseDirectory: string;
  canOpenDirectory: boolean;
}

export type AppSettingKey =
  | "applicationLanguage"
  | "fontSize"
  | "theme"
  | "defaultFolderPath"
  | "logDirectory"
  | "networkProxyUrl"
  | "networkProxyBypass"
  | "launchOnStartup"
  | "defaultPolicyTemplate";

export interface AppSettings {
  applicationLanguage: AppLanguage;
  fontSize: AppFontSize;
  theme: UcdThemeId;
  defaultFolderPath: string;
  logDirectory: string;
  networkProxyUrl: string;
  networkProxyBypass: string;
  launchOnStartup: boolean;
  defaultPolicyTemplate: PolicyTemplateName;
  loggingPolicy: LoggingPolicy;
}

export interface NodeInfo {
  available: boolean;
  path: string | null;
  version: string | null;
  reason: string | null;
}

export interface ClientLogEvent {
  level: LogLevel;
  kind: ClientLogEventKind;
  message: string;
  source: string;
  details?: Record<string, string>;
  stack?: string;
}
