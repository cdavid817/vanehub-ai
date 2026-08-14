import { describe, expect, it } from "vitest";
import { SLASH_COMMANDS } from "../services/slash-commands/command-catalog";
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

  // The test above is en-derived: a key missing from `en` itself never enters `keys`, so a
  // command shipped without its description key anywhere is invisible to it — absent from `en`,
  // absent from the rest, suite green. This test is catalog-derived instead, so it catches exactly
  // that gap (it already happened once: Tasks 4 and 5 shipped nine commands without keys).
  it("gives every catalogued command a description key in every locale", () => {
    for (const command of SLASH_COMMANDS) {
      const key = `slash.command.${command.name}.description`;
      for (const [locale, messages] of Object.entries(locales)) {
        expect(messages, `${locale} is missing ${key}`).toHaveProperty(key);
        expect(String(messages[key as keyof typeof messages]).trim(), `${locale}:${key}`).not.toBe("");
      }
    }
  });

  // findCommand (command-registry.ts) stops at the first name/alias match and returns null if
  // that one's `appliesTo` is false — it never checks whether a later entry would also match. A
  // duplicate name or alias would therefore surface to the user as "unknown command" rather than
  // as a loud failure, so uniqueness has to be asserted directly.
  it("has no duplicate command name or alias in the assembled catalog", () => {
    const seen = new Set<string>();
    for (const command of SLASH_COMMANDS) {
      for (const identifier of [command.name, ...(command.aliases ?? [])]) {
        expect(seen.has(identifier), `duplicate command identifier: ${identifier}`).toBe(false);
        seen.add(identifier);
      }
    }
  });
});
