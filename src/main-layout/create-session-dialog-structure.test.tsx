// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { SessionSeatAssignment } from "./session-seat-assignment";
import type { AgentWithModelFamily } from "../services/agent-model-family";
import type { ExpertRole } from "../types/agent-seats";

const agents: AgentWithModelFamily[] = [
  {
    id: "codex-cli",
    displayName: "Codex CLI",
    provider: "OpenAI",
    launch: { kind: "cli", executableName: "codex" },
    supportedInteractionModes: ["cli"],
    availabilityState: "available",
    capabilityTags: [],
    agentOrigin: "builtin",
    modelFamily: "openai",
  },
  {
    id: "claude-code",
    displayName: "Claude Code",
    provider: "Anthropic",
    launch: { kind: "cli", executableName: "claude" },
    supportedInteractionModes: ["cli"],
    availabilityState: "available",
    capabilityTags: [],
    agentOrigin: "builtin",
    modelFamily: "anthropic",
  },
];

const reviewer: ExpertRole = {
  id: "reviewer",
  displayName: "评审",
  avatar: "🔍",
  responsibility: "复核上一个席位的产出",
  reviewPolicy: { requireDifferentFamily: true },
} as ExpertRole;

describe("multi-Agent seat editor", () => {
  it("gives each seat a position, an Agent identity, and a per-seat accessible name", () => {
    render(
      <SessionSeatAssignment
        agents={agents}
        onSeatsChange={vi.fn()}
        roles={[reviewer]}
        seats={[
          { agentId: "codex-cli", roleId: null },
          { agentId: "claude-code", roleId: "reviewer" },
        ]}
      />,
    );

    // Every seat used to answer to the same two accessible names, so a screen reader could not
    // tell which pair of selects belonged to which participant.
    expect(screen.getByLabelText("席位 1 Agent")).toBeTruthy();
    expect(screen.getByLabelText("席位 2 角色")).toBeTruthy();

    const rows = document.querySelectorAll("div.ucd-list-row");
    expect(rows).toHaveLength(2);
    // The summary line, not the option list: it names the participant before the selects are opened.
    expect(within(rows[0] as HTMLElement).getAllByText(/不指定角色/).length).toBeGreaterThan(1);
    expect(within(rows[0] as HTMLElement).getAllByText(/Codex CLI/).length).toBeGreaterThan(1);
    expect(within(rows[1] as HTMLElement).getAllByText(/评审/).length).toBeGreaterThan(0);
  });

  it("states the cross-family constraint next to the seat it constrains", () => {
    render(
      <SessionSeatAssignment
        agents={agents}
        onSeatsChange={vi.fn()}
        roles={[reviewer]}
        seats={[
          { agentId: "codex-cli", roleId: null },
          { agentId: "claude-code", roleId: "reviewer" },
        ]}
      />,
    );

    const rows = document.querySelectorAll("div.ucd-list-row");
    // A reviewer seat is judged against the seat above it, so the explanation belongs on the
    // second seat and nowhere else.
    expect(within(rows[1] as HTMLElement).getByText(/不同模型家族/)).toBeTruthy();
    expect(within(rows[0] as HTMLElement).queryByText(/不同模型家族/)).toBeNull();
  });
});
