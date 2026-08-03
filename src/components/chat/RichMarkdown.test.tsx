// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { RichMarkdown } from "./RichMarkdown";
import { safeImageSource } from "./SafeImage";

describe("RichMarkdown", () => {
  it("renders GFM, math, and highlighted fenced code", () => {
    const { container } = render(
      <RichMarkdown>{[
        "~~obsolete~~",
        "",
        "| Name | Value |",
        "| --- | --- |",
        "| answer | 42 |",
        "",
        "- [x] verified",
        "",
        "Inline math: $E = mc^2$",
        "",
        "```javascript",
        "const answer = 42;",
        "```",
      ].join("\n")}</RichMarkdown>,
    );

    expect(container.querySelector("del")?.textContent).toBe("obsolete");
    expect(container.querySelector("table")?.textContent).toContain("answer");
    expect((container.querySelector('input[type="checkbox"]') as HTMLInputElement).disabled).toBe(true);
    expect(container.querySelector(".katex")?.textContent).toContain("mc2");
    expect(container.querySelector("code.hljs.language-javascript .hljs-keyword")?.textContent).toBe("const");
  });

  it("keeps raw HTML inactive", () => {
    const { container } = render(<RichMarkdown>{"before<script>window.bad = true</script>after"}</RichMarkdown>);

    expect(container.querySelector("script")).toBeNull();
    expect(container.textContent).toContain("<script>window.bad = true</script>");
  });

  it("rejects unsafe image sources", () => {
    render(<RichMarkdown>{"![unsafe](javascript:alert(1))"}</RichMarkdown>);

    expect(screen.getByText("图片无法显示")).not.toBeNull();
    expect(safeImageSource("http://example.com/image.png")).toBeNull();
    expect(safeImageSource("javascript:alert(1)")).toBeNull();
  });

  it("loads HTTPS images safely and opens an accessible preview", () => {
    render(<RichMarkdown>{"![Architecture](https://example.com/architecture.png)"}</RichMarkdown>);

    const image = screen.getByRole("img", { name: "Architecture" });
    expect(image.getAttribute("loading")).toBe("lazy");
    expect(image.getAttribute("referrerpolicy")).toBe("no-referrer");

    fireEvent.click(screen.getByRole("button", { name: "预览图片：Architecture" }));
    expect(screen.getByRole("dialog", { name: "图片预览" })).not.toBeNull();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "图片预览" })).toBeNull();
  });

  it("shows a fallback when an allowed image fails to load", () => {
    render(<RichMarkdown>{"![Broken](https://example.com/broken.png)"}</RichMarkdown>);

    fireEvent.error(screen.getByRole("img", { name: "Broken" }));
    expect(screen.getByText("图片无法显示")).not.toBeNull();
  });
});
