import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { normalizeLspServerDiscoveries } from "./lsp-contract";
import { lspSafeReasonCodes } from "../types/lsp";
import en from "../i18n/locales/en.json";
import ja from "../i18n/locales/ja.json";
import ko from "../i18n/locales/ko.json";
import zhCN from "../i18n/locales/zh-CN.json";
import zhTW from "../i18n/locales/zh-TW.json";

/**
 * Every reason code the backend can put on the wire has to be one this side accepts.
 *
 * The contract validator refuses an unknown code by throwing, and that throw takes down the whole
 * configuration query rather than one card -- so a code added natively and forgotten here turns an
 * actionable "install directory is not set" into "could not load settings". That is not a
 * hypothetical: four codes shipped with the Java entry and were missing here until this test.
 *
 * The Rust enum is read rather than restated. A second hand-written list would drift in exactly
 * the way this test exists to prevent.
 */
function backendReasonCodes(): string[] {
  // Git may materialize the Rust source with CRLF on Windows, while the parser below is line based.
  const source = readFileSync(
    "src-tauri/src/commands/code_intelligence/dto.rs",
    "utf8",
  ).replaceAll("\r\n", "\n");
  const body = /enum LspSafeReasonCodeDto \{\n(?<variants>[\s\S]*?)\n\}/u.exec(source)?.groups
    ?.variants;
  if (body === undefined) throw new Error("LspSafeReasonCodeDto not found in dto.rs");
  const variants = [...body.matchAll(/^\s{4}(?<name>[A-Z][A-Za-z0-9]*),$/gmu)]
    .map((match) => match.groups?.name ?? "");
  // `#[serde(rename_all = "snake_case")]` is what the wire actually carries.
  return variants.map((name) => name.replace(/(?<!^)[A-Z]/gu, (letter) => `_${letter}`).toLowerCase());
}

const codes = backendReasonCodes();

describe("LSP safe reason code parity", () => {
  it("finds the native vocabulary rather than silently comparing nothing", () => {
    expect(codes.length).toBeGreaterThan(20);
    expect(codes).toContain("install_directory_not_set");
  });

  it("declares exactly the vocabulary the command boundary can emit", () => {
    expect([...lspSafeReasonCodes].sort()).toEqual([...codes].sort());
  });

  // A code with no string renders as the raw code, which is the same drift one step later: the
  // user reads `install_directory_not_set` instead of a sentence telling them what to fill in.
  it.each(Object.entries({ en, "zh-CN": zhCN, "zh-TW": zhTW, ja, ko }))(
    "has a %s string for every reason code",
    (language, bundle: Record<string, string>) => {
      const missing = codes.filter((code) => {
        const value = bundle[`lspSettings.reason.${code}`];
        return typeof value !== "string" || value.trim() === "";
      });

      expect(missing, `${language} is missing`).toEqual([]);
    },
  );

  it.each(codes)("accepts %s on an unavailable discovery", (reasonCode) => {
    const discoveries = normalizeLspServerDiscoveries([{
      language: "java",
      server: "jdtls",
      availability: "unavailable",
      executablePath: null,
      arguments: [],
      reasonCode,
    }]);

    expect(discoveries[0]?.reasonCode).toBe(reasonCode);
  });
});
