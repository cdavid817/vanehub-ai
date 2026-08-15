// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import type { SlashCommand } from "./types";

// A non-async `run` that throws before ever returning a promise. Every shipped command is
// declared `async`, so this is the only way to reach dispatch's synchronous-throw guard without
// editing a real command module.
const throwingCommand = vi.hoisted(
  (): SlashCommand => ({
    name: "detonate",
    category: "info",
    appliesTo: () => true,
    run: () => {
      throw new Error("synchronous boom");
    },
  }),
);

// `SLASH_COMMANDS` is imported inside the hook, so the only way to give `dispatch` a fake command
// is to mock the catalog module. Extending the real export (instead of replacing it) keeps every
// other test in this file running against the actual command set.
vi.mock("./command-catalog", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./command-catalog")>();
  return { ...actual, SLASH_COMMANDS: [...actual.SLASH_COMMANDS, throwingCommand] };
});

import { useSlashCommands } from "./use-slash-commands";

const session = (agentId = "onepiece"): Session =>
  ({ id: "session-1", title: "S", agentId, interactionMode: "api" } as Session);

const config: ChatConfig = {
  agentId: "onepiece", interactionMode: "api", executionMode: "plan",
  streaming: true, thinking: false, longContext: false, reasoningDepth: "low",
};

function setup(overrides: {
  session?: Session | null;
  isStreaming?: boolean;
  openAssociatedPlan?: () => void;
} = {}) {
  const chat = {
    setSessionExecutionMode: vi.fn(),
    setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
  };
  const actions = {
    exportSession: vi.fn(),
    loadUsageSummary: vi.fn().mockResolvedValue({ totalTokens: 1, inputTokens: 1, outputTokens: 0, responseCount: 1 }),
  };
  const navigate = {
    openAssociatedPlan: overrides.openAssociatedPlan ?? null,
    openDestination: vi.fn(),
    openSessionTab: vi.fn(),
  };
  const onError = vi.fn();
  const rendered = renderHook(() => useSlashCommands({
    session: overrides.session === undefined ? session() : overrides.session,
    config, isStreaming: overrides.isStreaming ?? false, chat, actions, navigate, onError,
  }));
  return { ...rendered, chat, actions, navigate, onError };
}

describe("useSlashCommands", () => {
  it("passes ordinary prose through", () => {
    const { result } = setup();
    expect(result.current.dispatch("hello")).toEqual({ kind: "message" });
  });

  it("unescapes a doubled slash into literal content", () => {
    const { result } = setup();
    expect(result.current.dispatch("//help")).toEqual({ kind: "literal", content: "/help" });
  });

  it("passes everything through when the session is not eligible", () => {
    const { result } = setup({ session: session("claude-code") });
    expect(result.current.dispatch("/help")).toEqual({ kind: "message" });
  });

  it("passes everything through when there is no session", () => {
    const { result } = setup({ session: null });
    expect(result.current.dispatch("/help")).toEqual({ kind: "message" });
  });

  it("runs a known command and keeps it away from the model", async () => {
    const { result, chat } = setup();
    act(() => { expect(result.current.dispatch("/mode execute")).toEqual({ kind: "handled" }); });
    expect(chat.setSessionExecutionMode).toHaveBeenCalledWith("execute");
    await waitFor(() => expect(result.current.output?.titleKey).toBe("slash.output.applied"));
  });

  it("reports an unknown command without forwarding it", async () => {
    const { result } = setup();
    act(() => { expect(result.current.dispatch("/nope")).toEqual({ kind: "handled" }); });
    await waitFor(() => expect(result.current.output).toEqual({
      titleKey: "slash.error.title", tone: "error",
      messages: [{ key: "slash.error.unknown", params: { command: "nope" } }],
    }));
  });

  it("dismisses output on request", async () => {
    const { result } = setup();
    act(() => { result.current.dispatch("/status"); });
    await waitFor(() => expect(result.current.output).not.toBeNull());
    act(() => { result.current.dismissOutput(); });
    expect(result.current.output).toBeNull();
  });

  it("suggests commands while a bare slash prefix is being typed", () => {
    const { result } = setup();
    expect(result.current.suggestionQuery).toBeNull();

    act(() => { result.current.updateSuggestions("/mod"); });
    expect(result.current.suggestionQuery).toBe("mod");
    expect(result.current.suggestions.map((entry) => entry.name)).toEqual(["mode"]);

    act(() => { result.current.updateSuggestions("/mode plan"); });
    expect(result.current.suggestionQuery).toBeNull();
    expect(result.current.suggestions).toEqual([]);
  });

  it("offers the full command list for a bare slash", () => {
    const { result } = setup();
    act(() => { result.current.updateSuggestions("/"); });
    expect(result.current.suggestionQuery).toBe("");
    expect(result.current.suggestions.length).toBeGreaterThan(0);
  });

  it("shows no suggestions for ordinary prose", () => {
    const { result } = setup();
    act(() => { result.current.updateSuggestions("just chatting, not a command"); });
    expect(result.current.suggestionQuery).toBeNull();
    expect(result.current.suggestions).toEqual([]);
  });

  it("closes the dropdown once a completed name leaves a trailing space", () => {
    const { result } = setup();
    act(() => { result.current.updateSuggestions("/mode"); });
    expect(result.current.suggestionQuery).toBe("mode");

    act(() => { result.current.updateSuggestions("/mode "); });
    expect(result.current.suggestionQuery).toBeNull();
    expect(result.current.suggestions).toEqual([]);
  });

  it("still suggests commands when the draft has leading whitespace", () => {
    const { result } = setup();
    act(() => { result.current.updateSuggestions("  /mod"); });
    expect(result.current.suggestionQuery).toBe("mod");
    expect(result.current.suggestions.map((entry) => entry.name)).toEqual(["mode"]);
  });

  it("never executes a command from updateSuggestions", () => {
    const { result, chat } = setup();
    act(() => { result.current.updateSuggestions("/mode execute"); });
    expect(chat.setSessionExecutionMode).not.toHaveBeenCalled();
    expect(result.current.output).toBeNull();
  });

  it("completes a draft into a ready-to-run invocation", () => {
    const { result } = setup();
    expect(result.current.completeDraft("mode")).toBe("/mode ");
  });

  it("offers /plan only when the session has an associated plan run", () => {
    const without = setup();
    act(() => { without.result.current.updateSuggestions("/pla"); });
    expect(without.result.current.suggestions.map((entry) => entry.name)).toEqual(["plans"]);

    const withPlan = setup({ openAssociatedPlan: () => undefined });
    act(() => { withPlan.result.current.updateSuggestions("/pla"); });
    expect(withPlan.result.current.suggestions.map((entry) => entry.name)).toEqual(["plan", "plans"]);
  });

  it("reports a handler that throws through onError", async () => {
    const { result, actions, onError } = setup();
    actions.exportSession.mockImplementation(() => { throw new Error("boom"); });
    act(() => { result.current.dispatch("/export"); });
    await waitFor(() => expect(onError).toHaveBeenCalledWith("SlashCommands.export", expect.any(Error)));
    expect(result.current.output?.tone).toBe("error");
  });

  it("keeps a synchronous throw from a non-async run from escaping dispatch", () => {
    const { result, onError } = setup();
    // No `act(async ...)`/`waitFor` here on purpose: the throw happens inside the call to
    // `command.run`, so the try/catch's recovery is already done by the time `dispatch` returns,
    // unlike the rejection above which only settles after a microtask.
    act(() => { expect(result.current.dispatch("/detonate")).toEqual({ kind: "handled" }); });
    expect(onError).toHaveBeenCalledWith("SlashCommands.detonate", expect.any(Error));
    expect(result.current.output).toEqual({
      titleKey: "slash.error.title", tone: "error",
      messages: [{ key: "slash.error.failed", params: { command: "detonate" } }],
    });
  });
});
