// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import { ScheduledTasksPanel } from "./scheduled-tasks-panel";

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    listScheduledTasks: vi.fn().mockResolvedValue([]),
    createScheduledTask: vi.fn(),
    setScheduledTaskEnabled: vi.fn(),
    deleteScheduledTask: vi.fn(),
  },
}));

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];

describe("ScheduledTasksPanel", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("loads the task list on mount without needing a dialog `open` prop", async () => {
    render(<ScheduledTasksPanel agents={agents} />);
    expect(await screen.findByText("No scheduled tasks yet.")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Create task/i })).toBeTruthy();
  });
});
