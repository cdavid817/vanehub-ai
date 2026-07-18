import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { i18n } from "../../i18n";
import { RichBlocks } from "./RichBlocks";

describe("RichBlocks", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("renders representative first-version block kinds", () => {
    const html = renderToStaticMarkup(
      <RichBlocks
        blocks={[
          { id: "card-1", kind: "card", v: 1, title: "Summary", bodyMarkdown: "Use `RichBlocks`.", tone: "success" },
          { id: "diff-1", kind: "diff", v: 1, filePath: "src/main.ts", diff: "-old\n+new" },
          { id: "list-1", kind: "checklist", v: 1, items: [{ id: "done", text: "Rendered", checked: true }] },
          { id: "file-1", kind: "file", v: 1, url: "https://example.com/report.md", fileName: "report.md" },
          { id: "audio-1", kind: "audio", v: 1, url: "https://example.com/audio.mp3", text: "Voice summary" },
          { id: "widget-1", kind: "html_widget", v: 1, html: "<p>Widget</p>", height: 120 },
          {
            id: "interactive-1",
            kind: "interactive",
            v: 1,
            interactiveType: "select",
            options: [{ id: "a", label: "Option A" }],
          },
        ]}
      />,
    );

    expect(html).toContain("Summary");
    expect(html).toContain("src/main.ts");
    expect(html).toContain("1/1 complete");
    expect(html).toContain("report.md");
    expect(html).toContain("Voice summary");
    expect(html).toContain("sandbox");
    expect(html).toContain("Interactive actions are not enabled");
  });
});
