// @vitest-environment jsdom

import { fireEvent, render, waitFor } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { RichBlocks } from "./RichBlocks";

describe("RichBlocks", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
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
    expect(html).toContain("Interactive actions are not enabled");
    // Task 10.14: html_widget is collapsed by default, so its <iframe> (recognizable here by the
    // `sandbox` attribute the original, always-expanded implementation emitted unconditionally)
    // must not be part of the initial, collapsed markup at all -- only its truthful title is.
    expect(html).toContain("HTML widget");
    expect(html).not.toContain("sandbox");
  });

  // Task 10.14: defer expensive collapsed Rich Blocks while retaining a truthful summary and
  // accessible state.
  describe("collapsed Rich Blocks", () => {
    it("does not mount the html_widget iframe while collapsed, and mounts it only once expanded", async () => {
      const { container } = render(
        <RichBlocks blocks={[{ id: "widget-1", kind: "html_widget", v: 1, html: "<p>Widget</p>", title: "My Widget" }]} />,
      );

      // Proves the claim by DOM presence, not by inspection: React never creates the <iframe> node
      // at all while `open` is false (see HtmlWidgetBlock's `{open ? <iframe ... /> : null}`), so
      // there is nothing in the DOM for a browser to start loading -- CSS-hiding an already-mounted
      // iframe would not have this property.
      expect(container.querySelector("iframe")).toBeNull();
      const summary = container.querySelector("summary");
      expect(summary?.tagName).toBe("SUMMARY");
      expect(summary?.textContent).toContain("My Widget");

      fireEvent.click(summary!);

      // <details>'s `toggle` event (which HtmlWidgetBlock's onToggle listens on to flip `open`) is
      // queued as a task per the HTML spec, not dispatched synchronously with the click -- confirmed
      // against this repo's own jsdom (30.0.1) before writing this assertion, so `waitFor` is load-
      // bearing here, not defensive boilerplate.
      await waitFor(() => expect(container.querySelector("iframe")).not.toBeNull());
      const iframe = container.querySelector("iframe")!;
      expect(iframe.getAttribute("srcdoc")).toBe("<p>Widget</p>");
      expect(iframe.getAttribute("title")).toBe("My Widget");
    });

    it("keeps diff/media_gallery content natively mounted (browser-hidden, not React-unmounted) behind a closed, truthful <details>", () => {
      const { container } = render(
        <RichBlocks
          blocks={[
            { id: "diff-1", kind: "diff", v: 1, filePath: "src/a.ts", diff: "line1\nline2\nline3" },
            {
              id: "gallery-1",
              kind: "media_gallery",
              v: 1,
              title: "Shots",
              items: [{ url: "https://example.com/a.png" }, { url: "https://example.com/b.png" }],
            },
          ]}
        />,
      );

      const detailsList = container.querySelectorAll("details");
      expect(detailsList).toHaveLength(2);
      detailsList.forEach((details) => expect(details.hasAttribute("open")).toBe(false));

      // Unlike html_widget, the diff <pre> and gallery <img>s are still real DOM nodes while
      // collapsed -- deferred via the browser's native `details:not([open])` hiding (no layout/
      // paint) plus SafeImage's pre-existing `loading="lazy"`, not via React unmounting them.
      expect(container.querySelector("pre")?.textContent).toBe("line1\nline2\nline3");
      const images = container.querySelectorAll("img");
      expect(images).toHaveLength(2);
      expect(images[0].getAttribute("loading")).toBe("lazy");

      const summaries = container.querySelectorAll("summary");
      expect(summaries[0].textContent).toContain("src/a.ts");
      expect(summaries[0].textContent).toContain("3 lines");
      expect(summaries[1].textContent).toContain("Shots");
      expect(summaries[1].textContent).toContain("2 images");
    });

    it("does not gate bounded, always-expanded block kinds behind a disclosure", () => {
      const { container } = render(
        <RichBlocks
          blocks={[
            { id: "card-1", kind: "card", v: 1, title: "Summary" },
            { id: "list-1", kind: "checklist", v: 1, items: [{ id: "a", text: "A" }] },
            { id: "file-1", kind: "file", v: 1, url: "https://example.com/r.md", fileName: "r.md" },
            { id: "audio-1", kind: "audio", v: 1, url: "https://example.com/a.mp3" },
            { id: "interactive-1", kind: "interactive", v: 1, interactiveType: "select", options: [] },
          ]}
        />,
      );

      expect(container.querySelector("details")).toBeNull();
    });
  });
});
