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
});
