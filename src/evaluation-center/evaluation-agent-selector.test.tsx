// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry } from "../types/agent";
import { EvaluationAgentSelector } from "./evaluation-agent-selector";
import { MAX_EVALUATION_AGENTS } from "./evaluation-agent-filters";

function buildAgent(id: string, displayName: string, overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return {
    id, displayName, provider: id, launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
    availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
    ...overrides,
  };
}

const CLAUDE = buildAgent("claude-code", "Claude Code", { capabilityTags: ["coding", "cli"] });
const CODEX = buildAgent("codex-cli", "Codex CLI", {
  provider: "OpenAI", capabilityTags: ["coding", "api"], availabilityState: "needs-auth", unavailableReason: "Sign in to OpenAI.",
});
const ONEPIECE = buildAgent("onepiece", "OnePiece", {
  provider: "VaneHub", capabilityTags: ["api", "native"], availabilityState: "unavailable", unavailableReason: "OnePiece requires provider configuration.",
});
const AGENTS = [CLAUDE, CODEX, ONEPIECE];

describe("EvaluationAgentSelector", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("lists every Agent and reports the initial selected summary", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[CLAUDE.id]} />);
    for (const agent of AGENTS) expect(screen.getByTestId(`evaluation-agent-${agent.id}`)).toBeTruthy();
    expect(screen.getByTestId("evaluation-selected-summary").textContent).toBe("1 Agent selected");
  });

  it("narrows the visible list by search text against name, id, and provider", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[]} />);
    fireEvent.change(screen.getByLabelText("Search Agents"), { target: { value: "openai" } });
    expect(screen.getByTestId(`evaluation-agent-${CODEX.id}`)).toBeTruthy();
    expect(screen.queryByTestId(`evaluation-agent-${CLAUDE.id}`)).toBeNull();
    expect(screen.queryByTestId(`evaluation-agent-${ONEPIECE.id}`)).toBeNull();
  });

  it("narrows the visible list by status", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[]} />);
    fireEvent.change(screen.getByLabelText("Status"), { target: { value: "unavailable" } });
    expect(screen.getByTestId(`evaluation-agent-${ONEPIECE.id}`)).toBeTruthy();
    expect(screen.queryByTestId(`evaluation-agent-${CLAUDE.id}`)).toBeNull();
    expect(screen.queryByTestId(`evaluation-agent-${CODEX.id}`)).toBeNull();
  });

  it("narrows the visible list by capability, offering only tags present in the given roster", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[]} />);
    const capabilitySelect = screen.getByLabelText("Capability") as HTMLSelectElement;
    expect(Array.from(capabilitySelect.options).map((option) => option.value)).toEqual(["all", "api", "cli", "coding", "native"]);
    fireEvent.change(capabilitySelect, { target: { value: "native" } });
    expect(screen.getByTestId(`evaluation-agent-${ONEPIECE.id}`)).toBeTruthy();
    expect(screen.queryByTestId(`evaluation-agent-${CLAUDE.id}`)).toBeNull();
    expect(screen.queryByTestId(`evaluation-agent-${CODEX.id}`)).toBeNull();
  });

  it("shows the empty state, not a stale row list, once filters match nothing", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[]} />);
    fireEvent.change(screen.getByLabelText("Search Agents"), { target: { value: "no-such-agent" } });
    expect(screen.getByText("No Agents match these filters.")).toBeTruthy();
  });

  it("select-visible reports exactly the currently-filtered set, not a merge with the prior selection", () => {
    const onSelectVisible = vi.fn();
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={onSelectVisible} onToggle={vi.fn()} selectedIds={[ONEPIECE.id]} />);
    fireEvent.change(screen.getByLabelText("Status"), { target: { value: "needs-auth" } });
    fireEvent.click(screen.getByTestId("evaluation-select-visible"));
    expect(onSelectVisible).toHaveBeenCalledWith([CODEX.id]);
  });

  it("reports a toggle by id when a row checkbox is clicked", () => {
    const onToggle = vi.fn();
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={onToggle} selectedIds={[]} />);
    fireEvent.click(screen.getByTestId(`evaluation-agent-${CLAUDE.id}`));
    expect(onToggle).toHaveBeenCalledWith(CLAUDE.id);
  });

  it("shows an incompatible Agent's real status and reason instead of hiding it, and shows neither for a compatible one", () => {
    render(<EvaluationAgentSelector agents={AGENTS} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={[]} />);
    expect(screen.getByTestId(`evaluation-agent-${CLAUDE.id}`)).toBeTruthy();
    expect(screen.queryByTestId(`evaluation-agent-${CLAUDE.id}-reason`)).toBeNull();
    const codexReason = within(screen.getByTestId(`evaluation-agent-${CODEX.id}-reason`));
    expect(codexReason.getByText("Sign-in required")).toBeTruthy();
    expect(codexReason.getByText("Sign in to OpenAI.")).toBeTruthy();
    const onepieceReason = within(screen.getByTestId(`evaluation-agent-${ONEPIECE.id}-reason`));
    expect(onepieceReason.getByText("Unavailable")).toBeTruthy();
    expect(onepieceReason.getByText("OnePiece requires provider configuration.")).toBeTruthy();
  });

  it("disables only the not-yet-selected checkboxes once the real 8-Agent cap is reached", () => {
    const roster = Array.from({ length: MAX_EVALUATION_AGENTS + 1 }, (_, index) => buildAgent(`agent-${index}`, `Agent ${index}`));
    const selected = roster.slice(0, MAX_EVALUATION_AGENTS).map((agent) => agent.id);
    render(<EvaluationAgentSelector agents={roster} onSelectVisible={vi.fn()} onToggle={vi.fn()} selectedIds={selected} />);
    for (const id of selected) expect((screen.getByTestId(`evaluation-agent-${id}`) as HTMLInputElement).disabled).toBe(false);
    const overflowCheckbox = screen.getByTestId(`evaluation-agent-${roster[MAX_EVALUATION_AGENTS].id}`) as HTMLInputElement;
    expect(overflowCheckbox.disabled).toBe(true);
  });
});
