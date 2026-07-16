import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

void i18n.use(initReactI18next).init({
  fallbackLng: "zh-CN",
  interpolation: {
    escapeValue: false,
  },
  lng: "zh-CN",
  resources: {
    en: {
      translation: en,
    },
    "zh-CN": {
      translation: zhCN,
    },
  },
});

export { i18n };
