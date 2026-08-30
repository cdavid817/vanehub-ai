import zhCN from "./locales/zh-CN.json";
import { activityTranslationResource } from "./activity-locale-data";

export type TranslationResource = Record<string, string>;

interface SupportedLocaleDefinition<Id extends string> {
  id: Id;
  labelKey: `basic.language.${Id}`;
  direction: "ltr" | "rtl";
  load: () => Promise<TranslationResource>;
}

export const supportedLocales = [
  {
    id: "zh-CN",
    labelKey: "basic.language.zh-CN",
    direction: "ltr",
    load: async () => zhCN,
  },
  {
    id: "en",
    labelKey: "basic.language.en",
    direction: "ltr",
    load: async () => (await import("./locales/en.json")).default,
  },
  {
    id: "zh-TW",
    labelKey: "basic.language.zh-TW",
    direction: "ltr",
    load: async () => (await import("./locales/zh-TW.json")).default,
  },
  {
    id: "ja",
    labelKey: "basic.language.ja",
    direction: "ltr",
    load: async () => (await import("./locales/ja.json")).default,
  },
  {
    id: "ko",
    labelKey: "basic.language.ko",
    direction: "ltr",
    load: async () => (await import("./locales/ko.json")).default,
  },
] as const satisfies readonly SupportedLocaleDefinition<string>[];

export type AppLanguage = (typeof supportedLocales)[number]["id"];

export const defaultAppLanguage: AppLanguage = "zh-CN";
export const appLanguages: readonly AppLanguage[] = supportedLocales.map(({ id }) => id);

export function isSupportedAppLanguage(value: unknown): value is AppLanguage {
  return typeof value === "string" && supportedLocales.some(({ id }) => id === value);
}

export function getSupportedLocale(language: AppLanguage) {
  const locale = supportedLocales.find(({ id }) => id === language);
  if (!locale) throw new Error(`Unsupported application language: ${language}`);
  return locale;
}

export async function loadLocaleResource(language: AppLanguage): Promise<TranslationResource> {
  return {
    ...await getSupportedLocale(language).load(),
    ...activityTranslationResource(language),
  };
}

export const defaultTranslationResource: TranslationResource = {
  ...zhCN,
  ...activityTranslationResource(defaultAppLanguage),
};
