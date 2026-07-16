import type { UcdThemeId } from "../theme/theme-registry";

export const appLanguages = ["zh-CN", "en"] as const;
export type AppLanguage = (typeof appLanguages)[number];

export const appFontSizes = ["12px", "14px", "16px", "18px"] as const;
export type AppFontSize = (typeof appFontSizes)[number];

export type AppSettingKey = "applicationLanguage" | "fontSize" | "theme" | "defaultFolderPath";

export interface AppSettings {
  applicationLanguage: AppLanguage;
  fontSize: AppFontSize;
  theme: UcdThemeId;
  defaultFolderPath: string;
}

export interface NodeInfo {
  available: boolean;
  path: string | null;
  version: string | null;
  reason: string | null;
}
