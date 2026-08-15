import { describe, expect, it } from "vitest";
import { highlightFileLines, languageForPath } from "./code-highlighting";

function tagBalance(html: string) {
  return (html.match(/<span[^>]*>/g) ?? []).length - (html.match(/<\/span>/g) ?? []).length;
}

describe("code highlighting", () => {
  it("resolves a language from the extension and falls back to none", () => {
    expect(languageForPath("src/main.rs")).toBe("rust");
    expect(languageForPath("src/App.tsx")).toBe("typescript");
    expect(languageForPath("a/b/config.YAML")).toBe("yaml");
    expect(languageForPath("notes.unknownext")).toBeNull();
    expect(languageForPath("noextension")).toBeNull();
  });

  it("covers the languages the common bundle omits but the allowlist admits", () => {
    expect(languageForPath("Dockerfile")).toBe("dockerfile");
    expect(languageForPath("scripts/Deploy.ps1")).toBe("powershell");
    expect(languageForPath("CMakeLists.cmake")).toBe("cmake");
    expect(languageForPath("Main.scala")).toBe("scala");
  });

  it("numbers lines from 1 and keeps one entry per source line", () => {
    const lines = highlightFileLines("a.rs", "let a = 1;\nlet b = 2;\nlet c = 3;");
    expect(lines).toHaveLength(3);
    expect(lines.map((line) => line.number)).toEqual([1, 2, 3]);
  });

  it("leaves every line's markup balanced when a span straddles newlines", () => {
    // A block comment highlights as one span across three lines; splitting on \n alone
    // would leave line 1 with an unclosed tag and line 3 with a stray closer.
    const lines = highlightFileLines("a.rs", "/* one\n   two\n   three */\nlet x = 1;");
    expect(lines).toHaveLength(4);
    for (const line of lines) {
      expect(tagBalance(line.html)).toBe(0);
    }
    expect(lines[1].html).toContain("two");
  });

  it("escapes content when no language matches", () => {
    const lines = highlightFileLines("a.unknownext", "<script>alert(1)</script>");
    expect(lines[0].html).toBe("&lt;script&gt;alert(1)&lt;/script&gt;");
    expect(lines[0].html).not.toContain("<script>");
  });

  it("handles empty and single-line content without throwing", () => {
    expect(highlightFileLines("a.rs", "")).toEqual([{ number: 1, html: "" }]);
    expect(highlightFileLines("a.rs", "let x = 1;")).toHaveLength(1);
  });

  it("does not throw on content that does not parse as its language", () => {
    expect(() => highlightFileLines("a.json", "this is not json at all {{{")).not.toThrow();
    // A mislabelled text file reaches this with content its extension cannot explain;
    // that must not surface as an exception in the dialog.
    expect(() => highlightFileLines("a.rs", "<<<<<<< HEAD not rust at all")).not.toThrow();
  });
});
