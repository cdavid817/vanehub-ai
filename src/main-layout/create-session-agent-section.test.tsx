import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { SessionAgentModeSelector } from "./session-agent-mode-selector";
import { CreateSessionAgentSection } from "./create-session-agent-section";
import type { AgentRegistryEntry } from "../types/agent";

const agent: AgentRegistryEntry = {
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
  launch: { kind: "cli", executableName: "codex" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: [],
};

describe("Create session agent selection", () => {
  it("shows single agent and disabled multi agent modes", () => {
    const html = renderToStaticMarkup(<SessionAgentModeSelector mode="multi" onModeChange={vi.fn()} />);

    expect(html).toContain("单 Agent");
    expect(html).toContain("多 Agent");
    expect(html).toContain("暂未实现");
    expect(html).toContain("aria-disabled=\"true\"");
  });

  it("renders a disabled agent picker for coming-soon multi-agent sessions", () => {
    const html = renderToStaticMarkup(
      <CreateSessionAgentSection agents={[agent]} disabled onAgentSelect={vi.fn()} selectedAgent={agent} />,
    );

    expect(html).toContain("Codex CLI");
    expect(html).toContain("aria-disabled=\"true\"");
    expect(html).toContain("cursor-not-allowed");
  });
});
