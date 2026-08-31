// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { SessionSeat } from "../types/agent";
import { SessionRuntimePanel, type SessionRuntimePanelContentProps } from "./session-runtime-panel";
import { WorkspaceEvidenceScopeProvider } from "./workspace-evidence-scope";

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ componentProps }: { componentProps: Record<string, unknown> }) => (
    <div data-seat-id={String(componentProps.seatId ?? "")} data-testid="lazy-runtime-surface" />
  ),
}));

const singleSeat: SessionSeat[] = [{ seatId: "seat-solo", agentId: "claude-code", roleId: null }];
const multiSeats: SessionSeat[] = [
  { seatId: "seat-planner", agentId: "claude-code", roleId: null },
  { seatId: "seat-builder", agentId: "codex-cli", roleId: null },
];

function baseProps(overrides: Partial<SessionRuntimePanelContentProps> = {}): SessionRuntimePanelContentProps {
  return {
    activeSession: null,
    badges: {},
    maximized: false,
    messages: [],
    messagesPartial: false,
    onMaximizedChange: () => undefined,
    onSelectSeat: () => undefined,
    recordsRevision: 0,
    roles: [],
    seats: singleSeat,
    selectedSeat: null,
    sessionId: "session-1",
    turnStatus: null,
    ...overrides,
  };
}

function mount(props: SessionRuntimePanelContentProps, sessionId: string | null = "session-1") {
  return render(
    <I18nextProvider i18n={i18n}>
      <WorkspaceEvidenceScopeProvider seatIds={props.seats.flatMap((s) => (s.seatId ? [s.seatId] : []))} sessionId={sessionId as never}>
        <SessionRuntimePanel {...props} />
      </WorkspaceEvidenceScopeProvider>
    </I18nextProvider>,
  );
}

describe("SessionRuntimePanel", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders all four runtime surfaces as tabs", () => {
    mount(baseProps());
    for (const label of ["Terminal History", "Shell", "Logs", "Traces"]) {
      expect(screen.getByRole("tab", { name: label })).toBeTruthy();
    }
  });

  async function openShellTab(user: ReturnType<typeof userEvent.setup>) {
    // The Runtime Panel only mounts a tab once it has actually been the active one — Shell's own
    // content (the gate or the attached surface) does not exist in the DOM until this happens.
    await user.click(screen.getByRole("tab", { name: "Shell" }));
  }

  it("attaches a single-seat session's shell directly, with no gate", async () => {
    const user = userEvent.setup();
    mount(baseProps({ seats: singleSeat }));
    await openShellTab(user);

    expect(screen.queryByTestId("shell-seat-gate")).toBeNull();
    expect(screen.getAllByTestId("lazy-runtime-surface").length).toBeGreaterThan(0);
  });

  // 8.13: a multi-Agent session's Shell is one interactive channel with one owner — it must not
  // attach until the reader says whose it is, unlike Terminal History or Logs' "all seats" default.
  it("blocks Shell behind a seat gate for a multi-seat session with no seat chosen", async () => {
    const user = userEvent.setup();
    mount(baseProps({ seats: multiSeats, selectedSeat: null }));
    await openShellTab(user);

    expect(screen.getByTestId("shell-seat-gate")).toBeTruthy();
    expect(screen.getByText(i18n.t("session.shellSeatGate.prompt"))).toBeTruthy();
  });

  it("attaches the shell once a concrete seat is chosen from the gate", async () => {
    const user = userEvent.setup();
    const onSelectSeat = vi.fn();
    mount(baseProps({ onSelectSeat, seats: multiSeats, selectedSeat: null }));
    await openShellTab(user);

    await user.click(within(screen.getByTestId("shell-seat-gate")).getByRole("button", { name: /codex-cli/ }));
    expect(onSelectSeat).toHaveBeenCalledWith(1);
  });

  it("does not gate Shell once a concrete seat is selected", async () => {
    const user = userEvent.setup();
    mount(baseProps({ seats: multiSeats, selectedSeat: 0 }));
    await openShellTab(user);

    expect(screen.queryByTestId("shell-seat-gate")).toBeNull();
    expect(screen.getAllByTestId("lazy-runtime-surface").length).toBeGreaterThan(0);
  });

  it("does not gate Shell for a session with only one seat, even without a selection", async () => {
    // effectiveSeatId always resolves null for a single-seat session (tab-scope.ts) — there is no
    // ambiguity to gate against.
    const user = userEvent.setup();
    mount(baseProps({ seats: singleSeat, selectedSeat: null }));
    await openShellTab(user);

    expect(screen.queryByTestId("shell-seat-gate")).toBeNull();
  });

  it("renders a badge for a surface with something to report", () => {
    mount(baseProps({ badges: { logs: { atLeast: true, count: 2, kind: "count", tone: "danger" } } }));
    const badge = screen.getByRole("tab", { name: "Logs" }).querySelector(".border-destructive");
    expect(badge?.textContent).toBe("≥2");
  });

  it("renders nothing for a surface with no badge", () => {
    mount(baseProps({ badges: { logs: { kind: "none" } } }));
    expect(screen.getByRole("tab", { name: "Logs" }).querySelector(".border-destructive")).toBeNull();
  });
});
