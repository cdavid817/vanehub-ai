// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import { ScheduledTaskCapabilityNotice } from "./scheduled-task-capability-notice";

function buildAgent(overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return { id: "codex-cli", displayName: "Codex CLI", supportedInteractionModes: ["cli"], availabilityState: "available", ...overrides } as AgentRegistryEntry;
}

describe("ScheduledTaskCapabilityNotice", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows no warning when the assigned Agent is present and available, but always shows the execution notice", () => {
    render(<ScheduledTaskCapabilityNotice agent={buildAgent()} agentId="codex-cli" />);
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByTestId("scheduled-task-execution-notice")).toBeTruthy();
  });

  // 19.6: an agent id that no longer resolves in the registry (e.g. an uninstalled CLI Agent) is a
  // stronger case than merely "unavailable" -- the task's own honest wording says so explicitly.
  it("flags a missing Agent (not found in the registry at all) with its own explicit message", () => {
    render(<ScheduledTaskCapabilityNotice agent={undefined} agentId="removed-agent" />);
    expect(screen.getByText(i18n.t("scheduledTasks.capability.agentMissing", { agentId: "removed-agent" }))).toBeTruthy();
  });

  it("prefers the agent's own unavailableReason when present", () => {
    render(<ScheduledTaskCapabilityNotice agent={buildAgent({ availabilityState: "unavailable", unavailableReason: "Needs provider configuration." })} agentId="codex-cli" />);
    expect(screen.getByText("Needs provider configuration.")).toBeTruthy();
  });

  it("falls back to the generic agentStatus label when there is no specific unavailableReason", () => {
    render(<ScheduledTaskCapabilityNotice agent={buildAgent({ availabilityState: "needs-auth" })} agentId="codex-cli" />);
    expect(screen.getByText(i18n.t("scheduledTasks.agentStatus.needs-auth"))).toBeTruthy();
  });
});
