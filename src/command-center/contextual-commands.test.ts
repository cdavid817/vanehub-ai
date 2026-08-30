import { describe, expect, it, vi } from "vitest";
import { CONTEXTUAL_COMMANDS } from "./contextual-commands";
import type { WorkbenchCommandContext } from "./command-center-types";

function context(overrides: Partial<WorkbenchCommandContext> = {}): WorkbenchCommandContext {
  return {
    location: { destination: "sessions", sessionId: null, creatingSession: false },
    navigate: vi.fn(),
    onOpenSettings: vi.fn(),
    onNewSession: vi.fn(),
    onToggleNavigation: vi.fn(),
    onToggleInspector: vi.fn(),
    onToggleFocusMode: vi.fn(),
    ...overrides,
  };
}

function command(id: string) {
  const found = CONTEXTUAL_COMMANDS.find((entry) => entry.id === id);
  if (!found) throw new Error(`no command registered with id ${id}`);
  return found;
}

describe("CONTEXTUAL_COMMANDS", () => {
  it("covers five distinct commands, each with a unique id", () => {
    expect(CONTEXTUAL_COMMANDS).toHaveLength(5);
    expect(new Set(CONTEXTUAL_COMMANDS.map((entry) => entry.id)).size).toBe(5);
  });

  it("new-session calls onNewSession and is available everywhere", () => {
    const ctx = context();
    command("new-session").run(ctx);
    expect(ctx.onNewSession).toHaveBeenCalledOnce();
    expect(command("new-session").isAvailable(context({ location: { destination: "runs", section: "attention", runId: undefined } }))).toBe(true);
  });

  it("open-settings calls onOpenSettings and is available everywhere", () => {
    const ctx = context();
    command("open-settings").run(ctx);
    expect(ctx.onOpenSettings).toHaveBeenCalledOnce();
    expect(command("open-settings").isAvailable(context({ location: { destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined } }))).toBe(true);
  });

  it.each(["toggle-navigation", "toggle-inspector", "toggle-focus-mode"])(
    "%s is only available on the Sessions destination",
    (id) => {
      expect(command(id).isAvailable(context({ location: { destination: "sessions", sessionId: null, creatingSession: false } }))).toBe(true);
      expect(command(id).isAvailable(context({ location: { destination: "plan", section: "board", viewId: undefined, workItemId: undefined } }))).toBe(false);
    },
  );

  it("toggle-navigation, toggle-inspector, and toggle-focus-mode each call their own distinct callback", () => {
    const ctx = context();
    command("toggle-navigation").run(ctx);
    command("toggle-inspector").run(ctx);
    command("toggle-focus-mode").run(ctx);
    expect(ctx.onToggleNavigation).toHaveBeenCalledOnce();
    expect(ctx.onToggleInspector).toHaveBeenCalledOnce();
    expect(ctx.onToggleFocusMode).toHaveBeenCalledOnce();
  });

  it("does not define a Runtime Panel toggle — no such feature exists yet to toggle", () => {
    expect(CONTEXTUAL_COMMANDS.some((entry) => entry.id.includes("runtime"))).toBe(false);
  });
});
