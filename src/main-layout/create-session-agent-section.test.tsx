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
  agentOrigin: "builtin",
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

  it("groups and disables unconfigured OnePiece with a configuration action", () => {
    const onepiece: AgentRegistryEntry = {
      id: "onepiece",
      displayName: "OnePiece",
      provider: "VaneHub",
      launch: { kind: "api" },
      supportedInteractionModes: ["api"],
      availabilityState: "needs-auth",
      unavailableReason: "API credential is required.",
      capabilityTags: ["api", "native"],
      agentOrigin: "builtin",
    };
    const html = renderToStaticMarkup(
      <CreateSessionAgentSection
        agents={[onepiece, agent]}
        onAgentSelect={vi.fn()}
        selectedAgent={agent}
      />,
    );

    expect(html).toContain("VaneHub 原生");
    expect(html).toContain("内置 CLI");
    expect(html.indexOf("内置 CLI")).toBeLessThan(html.indexOf("VaneHub 原生"));
    expect(html).toContain("API credential is required.");
    expect(html).toContain("配置 OnePiece");
    expect(html).toContain("aria-disabled=\"true\"");
    expect(html).toContain("grid min-w-0 grid-cols-1 gap-2");
    expect(html).toContain("w-full min-w-0");
    expect(html).not.toContain("sm:grid-cols-2");
  });

  it("does not disable a CLI agent when only its optional SDK is missing", () => {
    const cliAgent: AgentRegistryEntry = {
      ...agent,
      availabilityState: "unavailable",
      managedSdkDependencyId: "codex-sdk",
      unavailableReason: "Managed SDK dependency 'codex-sdk' is not installed.",
    };
    const html = renderToStaticMarkup(
      <CreateSessionAgentSection
        agents={[cliAgent]}
        onAgentSelect={vi.fn()}
        selectedAgent={cliAgent}
      />,
    );

    expect(html).not.toContain("aria-disabled=\"true\"");
    expect(html).not.toContain("Managed SDK dependency");
    expect(html).not.toContain("cursor-not-allowed");
  });
});
