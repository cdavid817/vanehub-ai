// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AgentBrandIcon } from "./agent-brand-icon";

describe("AgentBrandIcon", () => {
  it("renders a dedicated accessible OnePiece vector icon", () => {
    const { container } = render(<AgentBrandIcon agentId="onepiece" title="OnePiece" />);

    expect(screen.getByTitle("OnePiece")).toBeTruthy();
    expect(container.querySelector('[data-agent-icon="onepiece"]')).toBeTruthy();
    expect(container.querySelector("svg")?.getAttribute("class")).toContain("h-4");
  });

  // Every expanded CLI carries its vendor's mark rather than the generic bot glyph, so a user
  // can tell the seven apart in the selector. Copilot is the one inline vector (Octicon, MIT).
  it("renders a vendor mark for each of the seven expanded CLIs", () => {
    for (const agentId of ["qwen-code", "kimi-cli", "qoder-cli", "codebuddy-code", "cursor-agent-cli", "iflow-cli"]) {
      const { container, unmount } = render(<AgentBrandIcon agentId={agentId} className="h-5 w-5" />);
      const mark = container.querySelector(`img[data-agent-icon="${agentId}"]`);
      expect(mark, agentId).toBeTruthy();
      expect(mark?.getAttribute("class")).toContain("h-5");
      // Decorative beside the Agent's own label: no alt text to double the accessible name.
      expect(mark?.getAttribute("alt")).toBe("");
      expect(mark?.getAttribute("aria-hidden")).toBe("true");
      unmount();
    }
    const titled = render(<AgentBrandIcon agentId="qwen-code" title="Qwen Code" />);
    expect(titled.container.querySelector("img")?.getAttribute("alt")).toBe("Qwen Code");
    titled.unmount();
    const { container } = render(<AgentBrandIcon agentId="copilot-cli" title="GitHub Copilot CLI" />);
    expect(container.querySelector('svg[data-agent-icon="copilot-cli"]')).toBeTruthy();
    expect(screen.getByTitle("GitHub Copilot CLI")).toBeTruthy();
  });

  it("keeps the generic glyph for an unknown Agent", () => {
    const { container } = render(<AgentBrandIcon agentId="someone-else" />);
    expect(container.querySelector("[data-agent-icon]")).toBeNull();
    expect(container.querySelector("svg")).toBeTruthy();
  });
});
