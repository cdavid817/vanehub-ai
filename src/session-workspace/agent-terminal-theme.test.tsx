// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { AgentTerminalEvent, AgentTerminalSession, Session } from "../types/agent";

class FakeTerminal {
  static instances: FakeTerminal[] = [];
  rows = 24;
  cols = 80;
  options: { theme?: Record<string, string> };
  written: string[] = [];
  disposed = false;
  scrollToBottomCalls = 0;
  clearCalls = 0;
  constructor(options: { theme?: Record<string, string> }) {
    this.options = { ...options };
    FakeTerminal.instances.push(this);
  }
  open() {}
  loadAddon() {}
  write(data: string) {
    this.written.push(data);
  }
  writeln(data: string) {
    this.written.push(`${data}\n`);
  }
  onData() {
    return { dispose() {} };
  }
  clear() {
    this.clearCalls += 1;
  }
  scrollToBottom() {
    this.scrollToBottomCalls += 1;
  }
  focus() {}
  dispose() {
    this.disposed = true;
  }
}

vi.mock("@xterm/xterm", () => ({ Terminal: FakeTerminal }));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

const openAgentTerminal = vi.fn<(sessionId: string) => Promise<AgentTerminalSession>>();
const stopAgentTerminal = vi.fn();
const subscribeAgentTerminalEvents = vi.fn<(sessionId: string, handler: (event: AgentTerminalEvent) => void) => Promise<() => void>>();
let emit: ((event: AgentTerminalEvent) => void) | null = null;

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    openAgentTerminal: (sessionId: string) => openAgentTerminal(sessionId),
    stopAgentTerminal: (...args: unknown[]) => stopAgentTerminal(...args),
    subscribeAgentTerminalEvents: (sessionId: string, handler: (event: AgentTerminalEvent) => void) => subscribeAgentTerminalEvents(sessionId, handler),
    sendAgentTerminalInput: vi.fn(async () => undefined),
    resizeAgentTerminal: vi.fn(async () => undefined),
  },
}));

// The palette is resolved from CSS, which jsdom does not load; the fake mirrors the contract that
// matters here — the background follows the root attribute and the element passed in is the host.
const createTerminalTheme = vi.fn((element?: Element) => ({
  background: document.documentElement.dataset.cliTerminalTheme === "light" ? "#ffffff" : "#0d1117",
  host: element?.getAttribute("aria-label") ?? "root",
}));
vi.mock("./terminal-theme", () => ({
  createTerminalTheme: (element?: Element) => createTerminalTheme(element),
  terminalFontFamily: "monospace",
}));

const { AgentTerminalTab } = await import("./agent-terminal-tab");

const session = { id: "session-1", agentId: "claude-code", title: "会话", interactionMode: "cli" } as unknown as Session;

function renderTab(isVisible = true, client = new QueryClient()) {
  return render(
    <QueryClientProvider client={client}>
      <AgentTerminalTab isVisible={isVisible} session={session} sessionActivationKey={1} />
    </QueryClientProvider>,
  );
}

describe("AgentTerminalTab CLI theme hot update", () => {

  beforeEach(() => {
    FakeTerminal.instances = [];
    emit = null;
    openAgentTerminal.mockReset();
    stopAgentTerminal.mockReset();
    subscribeAgentTerminalEvents.mockReset();
    createTerminalTheme.mockClear();
    delete document.documentElement.dataset.cliTerminalTheme;
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
      unobserve() {}
    };
    openAgentTerminal.mockResolvedValue({
      terminalId: "terminal-1",
      sessionId: "session-1",
      agentId: "claude-code",
      state: "running",
      capability: "native",
      size: { rows: 24, cols: 80 },
      runtimeSessionId: null,
      retained: true,
    });
    subscribeAgentTerminalEvents.mockImplementation(async (_sessionId, handler) => {
      emit = handler;
      return () => undefined;
    });
  });

  afterEach(() => {
    delete document.documentElement.dataset.cliTerminalTheme;
  });

  it("repaints in place: a new theme object, no new terminal, no reconnect, no lost output", async () => {
    document.documentElement.dataset.cliTerminalTheme = "dark";
    const view = renderTab();
    await waitFor(() => expect(openAgentTerminal).toHaveBeenCalledTimes(1));
    const terminal = FakeTerminal.instances[0];
    expect(FakeTerminal.instances).toHaveLength(1);
    expect(terminal.options.theme?.background).toBe("#0d1117");
    // Built from the host element, not the document root, so the scoped palette is what xterm sees.
    expect(terminal.options.theme?.host).toBe("Agent CLI 工作区");

    act(() => emit?.({ type: "output", terminalId: "terminal-1", sessionId: "session-1", content: "hello from cli" }));
    await waitFor(() => expect(terminal.written.join("")).toContain("hello from cli"));
    const themeBefore = terminal.options.theme;

    document.documentElement.dataset.cliTerminalTheme = "light";
    await waitFor(() => expect(terminal.options.theme?.background).toBe("#ffffff"));

    expect(terminal.options.theme).not.toBe(themeBefore);
    expect(FakeTerminal.instances).toHaveLength(1);
    expect(terminal.disposed).toBe(false);
    expect(openAgentTerminal).toHaveBeenCalledTimes(1);
    expect(subscribeAgentTerminalEvents).toHaveBeenCalledTimes(1);
    expect(stopAgentTerminal).not.toHaveBeenCalled();
    expect(terminal.clearCalls).toBe(0);
    expect(terminal.scrollToBottomCalls).toBe(0);
    expect(terminal.written.join("")).toContain("hello from cli");
    expect(view.getByRole("button", { name: "停止" })).toBeTruthy();
  });

  it("applies the current setting to a hidden terminal so it is right when revealed, and to a new one", async () => {
    document.documentElement.dataset.cliTerminalTheme = "dark";
    const client = new QueryClient();
    const view = renderTab(false, client);
    await waitFor(() => expect(openAgentTerminal).toHaveBeenCalledTimes(1));
    const hidden = FakeTerminal.instances[0];

    document.documentElement.dataset.cliTerminalTheme = "light";
    await waitFor(() => expect(hidden.options.theme?.background).toBe("#ffffff"));

    view.rerender(
      <QueryClientProvider client={client}>
        <AgentTerminalTab isVisible session={session} sessionActivationKey={1} />
      </QueryClientProvider>,
    );
    expect(FakeTerminal.instances).toHaveLength(1);
    expect(hidden.options.theme?.background).toBe("#ffffff");

    view.unmount();
    renderTab();
    await waitFor(() => expect(FakeTerminal.instances).toHaveLength(2));
    expect(FakeTerminal.instances[1].options.theme?.background).toBe("#ffffff");
  });

  it("does not react to the application theme changing the CLI palette or vice versa", async () => {
    document.documentElement.dataset.cliTerminalTheme = "light";
    document.documentElement.dataset.theme = "futuristic";
    renderTab();
    await waitFor(() => expect(openAgentTerminal).toHaveBeenCalledTimes(1));
    const terminal = FakeTerminal.instances[0];
    expect(terminal.options.theme?.background).toBe("#ffffff");

    const callsBefore = createTerminalTheme.mock.calls.length;
    document.documentElement.dataset.theme = "minimal";
    // Give a would-be observer callback a chance to run; the CLI palette is not derived from
    // the application theme, so nothing should be rebuilt.
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(createTerminalTheme.mock.calls.length).toBe(callsBefore);
    expect(terminal.options.theme?.background).toBe("#ffffff");
    expect(document.documentElement.dataset.cliTerminalTheme).toBe("light");
  });

  it("coalesces resize notifications and only sends a changed grid", async () => {
    const observer: { fire: (() => void) | null } = { fire: null };
    globalThis.ResizeObserver = class {
      constructor(callback: ResizeObserverCallback) {
        observer.fire = () => callback([], this as unknown as ResizeObserver);
      }
      observe() {}
      disconnect() {}
      unobserve() {}
    } as unknown as typeof ResizeObserver;
    const { agentService } = await import("../services/runtime-agent-client");
    const resize = agentService.resizeAgentTerminal as unknown as ReturnType<typeof vi.fn>;
    resize.mockClear();
    renderTab();
    await waitFor(() => expect(openAgentTerminal).toHaveBeenCalledTimes(1));
    const terminal = FakeTerminal.instances[0];
    // The reveal effect schedules one unconditional resize on the next frame; let it land first.
    await new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));
    const callsAfterConnect = resize.mock.calls.length;

    // Ten observer ticks in one frame with the same 24x80 grid: nothing new to tell the PTY.
    for (let index = 0; index < 10; index += 1) observer.fire?.();
    await new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));
    expect(resize.mock.calls.length).toBe(callsAfterConnect);

    // A real grid change goes through exactly once.
    terminal.rows = 40;
    observer.fire?.();
    observer.fire?.();
    await new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));
    expect(resize.mock.calls.length).toBe(callsAfterConnect + 1);
    expect(resize.mock.calls.at(-1)?.[1]).toEqual({ rows: 40, cols: 80 });
  });
});
