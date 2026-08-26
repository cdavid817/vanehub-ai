/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../i18n";
import { RichMarkdown } from "../components/chat/RichMarkdown";

/**
 * Two guards, and they answer different questions.
 *
 * One feeds a hostile document to the renderer the workspace panels use and checks that nothing in
 * it becomes active. The other reads the panel sources and checks that no new way of putting markup
 * into the DOM has appeared — because the first can only fail for the sinks somebody thought to
 * test, and the second catches the one nobody did.
 *
 * The matching panel-level case lives with the Documents suite rather than here. Importing this
 * renderer at module scope pulls in Mermaid and KaTeX, and doing that alongside a full panel render
 * left the panel's own queries never running — a test-harness interaction, not a product fault, and
 * not one worth debugging when the panel suite already renders the panel correctly.
 */

const HOSTILE = [
  "# Title",
  "",
  "<script>window.__owned = true</script>",
  "",
  '<img src=x onerror="window.__owned = true">',
  "",
  "[link](javascript:window.__owned=true)",
  "",
  "```mermaid",
  "graph TD; A-->B;",
  "```",
  "",
  "Inline math: $E = mc^2$",
].join("\n");

describe("workspace Markdown safety", () => {
  it("leaves script and event-handler markup inert", () => {
    const { container } = render(<RichMarkdown>{HOSTILE}</RichMarkdown>);

    // Not parsed at all: `rehype-raw` is deliberately absent, so the markup arrives as text rather
    // than as an element that was then sanitised. Sanitising is a filter somebody has to keep
    // right; not parsing is a property of the arrangement.
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img[onerror]")).toBeNull();
    expect((window as unknown as { __owned?: boolean }).__owned).toBeUndefined();
  });

  it("refuses a javascript: link", () => {
    const { container } = render(<RichMarkdown>{HOSTILE}</RichMarkdown>);

    const href = container.querySelector("a")?.getAttribute("href") ?? "";
    // react-markdown's default URL transform drops it. A link that navigated to a script would be
    // the same hole as a `<script>`, arriving through a form that looks like ordinary content.
    expect(href.startsWith("javascript:")).toBe(false);
  });

  it("keeps math and diagrams working while it does so", () => {
    const { container } = render(<RichMarkdown>{HOSTILE}</RichMarkdown>);

    // The point of reusing the renderer rather than writing a stricter one: safety that also took
    // away math and diagrams would be traded back the first time somebody needed them.
    expect(container.querySelector(".katex")).toBeTruthy();
  });

  it("introduces no new raw-markup sink in the workspace panels", () => {
    // Through `fileURLToPath` rather than `URL.pathname`: on Windows the latter yields `/D:/...`,
    // which `readdirSync` reads as a path on the root of the current drive.
    const directory = dirname(fileURLToPath(import.meta.url));
    const offenders = readdirSync(directory)
      .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test."))
      .filter((name) => {
        const source = readFileSync(join(directory, name), "utf8");
        // Matched on the forms that actually create a sink rather than on the words. Scanning for
        // the bare word `rehype-raw` flagged the comment explaining why it is absent — the same
        // crude-substring mistake that would eventually be "fixed" by deleting the explanation.
        return [
          /dangerouslySetInnerHTML\s*=/,
          /\binnerHTML\s*=/,
          /from\s+["']rehype-raw["']/,
          /require\(\s*["']rehype-raw["']\s*\)/,
        ].some((sink) => sink.test(source));
      });

    // Zero, and it has to stay zero. The one legitimate raw-markup sink this panel relies on is
    // `PreviewLineRow`, which lives with the chat components and injects highlight.js output —
    // markup that escapes the text it wraps. A second sink here would be a second thing to argue
    // about, and the argument would be lost quietly.
    expect(offenders).toEqual([]);
  });
});
