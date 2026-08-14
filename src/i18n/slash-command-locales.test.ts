import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

const locales = { en, ja, ko, zhCN, zhTW };

describe("slash command localization", () => {
  it("keeps every English slash key present and non-empty in all supported locales", () => {
    const keys = Object.keys(en).filter((key) => key.startsWith("slash."));
    expect(keys.length).toBeGreaterThan(0);

    for (const [locale, messages] of Object.entries(locales)) {
      for (const key of keys) {
        expect(messages, `${locale} is missing ${key}`).toHaveProperty(key);
        expect(String(messages[key as keyof typeof messages]).trim(), `${locale}:${key}`).not.toBe("");
      }
    }
  });
});
