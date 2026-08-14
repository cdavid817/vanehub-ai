import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

const locales = { en, ja, ko, zhCN, zhTW };
const prefixes = [
  "onepiece.tools.",
  "sessionTabs.artifacts.",
  "sessionTabs.browserHandoff.",
  "sessionTabs.delegation.",
  "sessionTabs.tools.",
];

describe("built-in tool localization", () => {
  it("keeps every new English key present in all supported locales", () => {
    const keys = Object.keys(en).filter((key) => prefixes.some((prefix) => key.startsWith(prefix)));
    expect(keys.length).toBeGreaterThan(50);

    for (const [locale, messages] of Object.entries(locales)) {
      for (const key of keys) {
        expect(messages, `${locale} is missing ${key}`).toHaveProperty(key);
        expect(String(messages[key as keyof typeof messages]).trim(), `${locale}:${key}`).not.toBe("");
      }
    }
  });
});
