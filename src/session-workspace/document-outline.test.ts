import { describe, expect, it } from "vitest";
import { documentOutline, MAX_OUTLINE_ENTRIES } from "./document-outline";

describe("documentOutline", () => {
  it("reads headings with their depth and line", () => {
    const outline = documentOutline("# Title\n\ntext\n\n## Section\n");

    expect(outline).toEqual([
      { depth: 1, text: "Title", line: 1, anchor: "title" },
      { depth: 2, text: "Section", line: 5, anchor: "section" },
    ]);
  });

  it("does not mistake shell comments in fenced code for headings", () => {
    const outline = documentOutline("# Real\n\n```sh\n# not a heading\n```\n\n## Also real\n");

    // A shell script in a Markdown block is full of `# comment` lines. An outline that listed them
    // would be a table of contents made mostly of other people's comments.
    expect(outline.map((entry) => entry.text)).toEqual(["Real", "Also real"]);
  });

  it("closes a fence opened with tildes", () => {
    const outline = documentOutline("~~~\n# hidden\n~~~\n\n# Visible\n");

    expect(outline.map((entry) => entry.text)).toEqual(["Visible"]);
  });

  it("gives repeated headings distinct anchors", () => {
    const outline = documentOutline("## Overview\n\n## Overview\n\n## Overview\n");

    // "Overview" under three sections is normal. Two headings sharing an anchor would scroll to the
    // first one every time, which reads as the second entry being broken.
    expect(outline.map((entry) => entry.anchor)).toEqual([
      "overview",
      "overview-1",
      "overview-2",
    ]);
  });

  it("keeps an anchor for a heading with no anchorable characters", () => {
    const outline = documentOutline("# ***\n");

    // Empty would collide with the next such heading and scroll nowhere. A name is better than a
    // blank even when the name carries nothing.
    expect(outline[0]?.anchor).toBe("section");
  });

  it("ignores a heading marker with nothing after it", () => {
    const outline = documentOutline("#\n\n#   \n\n# Real\n");

    expect(outline.map((entry) => entry.text)).toEqual(["Real"]);
  });

  it("strips closing hashes rather than putting them in the outline", () => {
    expect(documentOutline("## Section ##\n")[0]?.text).toBe("Section");
  });

  it("stops at its bound", () => {
    const many = Array.from({ length: MAX_OUTLINE_ENTRIES + 50 }, (_, index) => `# H${index}`).join(
      "\n",
    );

    // A long document has more headings than a reader can use as a list, and the outline is a
    // navigation aid rather than a second copy of the document.
    expect(documentOutline(many)).toHaveLength(MAX_OUTLINE_ENTRIES);
  });

  it("has nothing to say about a document with no headings", () => {
    expect(documentOutline("just some prose\n")).toEqual([]);
  });
});
