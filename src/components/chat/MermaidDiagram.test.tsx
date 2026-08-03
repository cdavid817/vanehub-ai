// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { MermaidDiagram } from "./MermaidDiagram";

vi.mock("mermaid", () => ({
  default: {
    initialize: vi.fn(),
    render: vi.fn().mockRejectedValue(new Error("invalid diagram")),
  },
}));

describe("MermaidDiagram", () => {
  it("preserves source when rendering fails", async () => {
    const chart = "graph TD\nA-->|broken|";
    const { container } = render(<MermaidDiagram chart={chart} />);

    expect(await screen.findByText("Mermaid 图表渲染失败")).not.toBeNull();
    expect(container.querySelector("pre code")?.textContent).toBe(chart);
  });
});
