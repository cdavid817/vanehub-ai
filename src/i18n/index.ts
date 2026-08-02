import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import {
  defaultAppLanguage,
  defaultTranslationResource,
  getSupportedLocale,
  loadLocaleResource,
  type AppLanguage,
  type TranslationResource,
} from "./supported-locales";

void i18n.use(initReactI18next).init({
  fallbackLng: defaultAppLanguage,
  interpolation: {
    escapeValue: false,
  },
  lng: defaultAppLanguage,
  resources: {
    [defaultAppLanguage]: {
      translation: defaultTranslationResource,
    },
  },
});

const resourceLoads = new Map<AppLanguage, Promise<void>>();

type LocaleResourceLoader = (language: AppLanguage) => Promise<TranslationResource>;

export async function ensureAppLanguage(language: AppLanguage, loadResource: LocaleResourceLoader = loadLocaleResource): Promise<void> {
  if (i18n.hasResourceBundle(language, "translation")) return;
  const existingLoad = resourceLoads.get(language);
  if (existingLoad) return existingLoad;

  const load = loadResource(language)
    .then((resource) => {
      i18n.addResourceBundle(language, "translation", resource, true, true);
    })
    .finally(() => {
      resourceLoads.delete(language);
    });
  resourceLoads.set(language, load);
  return load;
}

export async function activateAppLanguage(language: AppLanguage, loadResource: LocaleResourceLoader = loadLocaleResource): Promise<void> {
  try {
    await ensureAppLanguage(language, loadResource);
    await i18n.changeLanguage(language);
    applyDocumentLocale(language);
  } catch (error) {
    await i18n.changeLanguage(defaultAppLanguage);
    applyDocumentLocale(defaultAppLanguage);
    const message = i18n.t("basic.languageLoadError", { language });
    throw new Error(message, { cause: error });
  }
}

function applyDocumentLocale(language: AppLanguage): void {
  if (typeof document === "undefined") return;
  document.documentElement.dir = getSupportedLocale(language).direction;
  document.documentElement.lang = language;
}

export { i18n };
