// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { i18n } from "../../i18n";
import { DocumentationPage, normalizeDocumentationMarkdown, resolveReadme } from "./documentation-page";

describe("resolveReadme", () => {
  it("selects a bundled translation by base language tag", () => {
    // zh-TW ships no README of its own; the base tag is what makes it resolve to Chinese rather
    // than falling all the way back to English.
    expect(resolveReadme("zh-CN")).toBe(resolveReadme("zh-TW"));
    expect(resolveReadme("zh-CN")).not.toBe(resolveReadme("en"));
    expect(resolveReadme("ja")).not.toBe(resolveReadme("en"));
  });

  it("falls back to English for a language with no bundled README", () => {
    expect(resolveReadme("ko")).toBe(resolveReadme("en-US"));
  });
});

describe("normalizeDocumentationMarkdown", () => {
  it("removes README presentation tags and resolves relative links to GitHub", () => {
    const markdown = normalizeDocumentationMarkdown('<div align="center">Hello</div>\n[Guide](docs/guide.md)');

    expect(markdown).toBe("Hello\n[Guide](https://github.com/cdavid817/vanehub-ai/blob/main/docs/guide.md)");
  });
});

describe("DocumentationPage", () => {
  it("renders the bundled README as document content without a network call", async () => {
    await i18n.changeLanguage("zh-CN");
    render(<DocumentationPage />);

    expect(screen.getByRole("heading", { level: 2, name: "使用文档" })).toBeTruthy();
    // The README's own top-level heading proves the raw import reached the markdown renderer.
    expect(screen.getByRole("heading", { name: "VaneHub AI" })).toBeTruthy();
  });
});
