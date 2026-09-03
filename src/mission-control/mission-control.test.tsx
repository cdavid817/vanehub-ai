// @vitest-environment jsdom

import type { ReactElement } from "react";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
  webAgentClient,
} from "../services/web-agent-client";
import { updateWebAgentRun } from "../services/web-agent-run-state";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlOverview, MissionControlRunDetail } from "../types/mission-control";
import { MissionControl } from "./mission-control";

const PAUSED_RUN_ID = "018f0f17-4d6a-7e20-b41d-66c5271a28d0";
const RUNNING_RUN_ID = "018f0f17-4d6a-7e20-b41d-66c5271a296";

// `sessionStorage.clear()`: mission-control-view-state.ts persists filters there (4.8), and
// jsdom's storage is one shared global across every test in this file — without clearing it, a
// filter set by an earlier test silently becomes a later test's initial state.
afterEach(() => { cleanup(); vi.restoreAllMocks(); resetWebMissionControlRunsForTest(); sessionStorage.clear(); localStorage.clear(); });

// 16.5/16.6: filters and Saved Views now live behind FilterPopover's/MissionControlSavedViewMenu's
// own trigger, not a permanently-visible grid. Idempotent rather than a bare click: both close on
// an outside *pointerdown*, which plain `fireEvent.click` never dispatches, so a naive second click
// on an already-open trigger would toggle it shut instead of being a harmless no-op — same pattern
// work-board.test.tsx's own `openByTrigger` established for the identical primitives.
function openByTrigger(name: string | RegExp) {
  const trigger = screen.getByRole("button", { name });
  if (trigger.getAttribute("aria-expanded") !== "true") fireEvent.click(trigger);
}

// 16.13: `RunCard` now renders a real `EvidenceLink` (a react-router `Link`) for any run whose
// "review" action has a resolvable session -- the small fixed Web demo fixture's own review-linked
// run (index 4) is present by default across most of this file's tests, not just the one that
// interacts with it directly, so every render needs a Router ancestor, not just that one test.
function withRouter(ui: ReactElement) {
  return <MemoryRouter>{ui}</MemoryRouter>;
}

function agentFixture(id: string, displayName: string): AgentRegistryEntry {
  return {
    id, displayName, provider: "test", launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
    availabilityState: "available", capabilityTags: [], agentOrigin: "user",
  };
}

// 20.3: mirrors projects.test.tsx's own identical helper (no shared version exists to import --
// each compact-layout test file defines its own copy, same established convention).
function stubMatchMedia(matches: (query: string) => boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: (query: string): MediaQueryList => ({
      matches: matches(query),
      media: query,
      onchange: null,
      addEventListener: () => undefined,
      addListener: () => undefined,
      dispatchEvent: () => false,
      removeEventListener: () => undefined,
      removeListener: () => undefined,
    }),
  });
}

describe("MissionControl", () => {
  it("renders a bounded page from 1,000 Web Runs and loads detail only on inspection", async () => {
    await i18n.changeLanguage("en");
    seedWebMissionControlRunsForTest(1_000);
    const detail = vi.spyOn(webAgentClient, "getMissionControlRun");

    render(withRouter(<MissionControl />));

    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']")).toHaveLength(60));
    expect(detail).not.toHaveBeenCalled();
    fireEvent.click(document.querySelector("[data-testid^='mission-run-'] button")!);
    // The tablist is what proves detail actually rendered, so it belongs inside the waitFor.
    // Asserting it synchronously after only awaiting the spy call raced: the spy fires when the
    // request is *issued*, leaving promise resolution and React's commit still pending — which a
    // loaded CI runner does not finish within the same tick. Verified by injecting a 50ms delay
    // into the detail call: this shape passes, the synchronous one fails on exactly this line.
    await waitFor(() => {
      expect(detail).toHaveBeenCalledOnce();
      expect(document.querySelector("[role='tablist']")).toBeTruthy();
    });
  });

  it("prioritizes attention, freezes terminal elapsed time, filters via the Toolbar/FilterPopover, inspects, and navigates", async () => {
    // `activateAppLanguage`, not the bare `i18n.changeLanguage`: only zh-CN is bundled at init
    // (supported-locales.ts's own `defaultAppLanguage`) -- the "en" resource bundle is loaded
    // lazily, and a bare `changeLanguage("en")` before it has ever been fetched silently falls
    // back to zh-CN for every `t()` call rather than throwing. The pre-existing test this replaces
    // got away with the bare form because its own `getByLabelText(/Runner/)` is a substring regex
    // against a label that happens to keep the English loanword "Runner" even in zh-CN
    // ("按 Runner 筛选") -- this test's own `/Filters/` ("筛选条件" in zh-CN, no shared substring)
    // does not have that same accidental immunity, so it needs the real fix instead.
    await activateAppLanguage("en"); const navigate = vi.fn();
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(withRouter(<MissionControl onNavigate={navigate} />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));
    expect(document.querySelector("[data-runner='ssh']")?.textContent).toContain("build.example.test");

    // 16.5: runner and status now live behind the Filters trigger, migrated verbatim from the old
    // always-visible <select>s -- same aria-labels, same option values.
    openByTrigger(/Filters/);
    fireEvent.change(screen.getByLabelText("Filter by Runner"), { target: { value: "ssh" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ runner: "ssh", cursor: null })));

    fireEvent.change(screen.getByLabelText("Filter by status"), { target: { value: "failed" } });
    const failed = await screen.findAllByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294");
    fireEvent.click(failed[0].querySelector("button")!);
    await waitFor(() => expect(document.querySelector("[role='tablist']")).toBeTruthy());
    // 16.13: "review" is now a real EvidenceLink (a react-router `Link` to the linked review's own
    // session), not an `onAct`/`onNavigate` trigger -- proven here via its real `href` the same way
    // EvidenceLink.test.tsx's own "links to the authoritative page" case does, not by asserting the
    // old callback fired (it no longer does; this action is pure declarative navigation now). The
    // `?returnTo=` token is this run's own real Attention-bucket location (`withReturnTo`, reusing
    // the same safe, validated mechanism `navigateFromMissionControl` already relies on for this
    // exact action) -- not fabricated, and not lost by moving off the old callback path.
    const reviewLink = within(failed[0]).getByRole("link", { name: "Review changes" });
    expect(reviewLink.getAttribute("href")).toBe(
      "/workspace/sessions/web-session-4?returnTo=%2Fworkspace%2Fruns%2Fattention%2F018f0f17-4d6a-7e20-b41d-66c5271a294",
    );
    expect(navigate).not.toHaveBeenCalled();
  });

  it("16.2: scopes rendering to one section's own RunSection when a route tab is given, and shows all three when it is not", async () => {
    await activateAppLanguage("en");
    const { rerender } = render(withRouter(<MissionControl section="attention" />));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Attention inbox" })).toBeTruthy());
    expect(screen.queryByRole("heading", { name: "Active Runs" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Recently completed" })).toBeNull();

    rerender(withRouter(<MissionControl section="active" />));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Active Runs" })).toBeTruthy());
    expect(screen.queryByRole("heading", { name: "Attention inbox" })).toBeNull();

    rerender(withRouter(<MissionControl />));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Attention inbox" })).toBeTruthy());
    expect(screen.getByRole("heading", { name: "Active Runs" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Recently completed" })).toBeTruthy();
  });

  it("does not let a slower inspect() response overwrite a more recently selected run's detail", async () => {
    await i18n.changeLanguage("en");
    seedWebMissionControlRunsForTest(100);
    render(withRouter(<MissionControl />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThanOrEqual(2));

    const rows = Array.from(document.querySelectorAll("[data-testid^='mission-run-']"));
    const firstRunId = rows[0].getAttribute("data-testid")!.replace("mission-run-", "");
    const secondRunId = rows[1].getAttribute("data-testid")!.replace("mission-run-", "");
    // Real, valid detail fixtures fetched through the underlying client, before the spy below
    // starts intercepting the very same method on `agentService`.
    const firstDetail = await webAgentClient.getMissionControlRun(firstRunId);
    const secondDetail = await webAgentClient.getMissionControlRun(secondRunId);

    let resolveFirst: ((detail: MissionControlRunDetail) => void) | undefined;
    let resolveSecond: ((detail: MissionControlRunDetail) => void) | undefined;
    vi.spyOn(agentService, "getMissionControlRun")
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));

    fireEvent.click(rows[0].querySelector("button")!);
    fireEvent.click(rows[1].querySelector("button")!);

    // The second (more recent) click's response arrives first, as it normally would.
    resolveSecond?.(secondDetail);
    await waitFor(() => expect(document.querySelector("aside [role='tablist']")).toBeTruthy());
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${secondRunId}']`)).toBeTruthy();

    // The stale first click's response arrives late — must not clobber the second run's detail.
    resolveFirst?.(firstDetail);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${secondRunId}']`)).toBeTruthy();
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${firstRunId}']`)).toBeNull();
  });

  it("shows safe errors and does not expose backend diagnostics", async () => {
    await i18n.changeLanguage("en");
    vi.spyOn(agentService, "getMissionControlOverview").mockRejectedValue(new Error("token=secret"));
    render(withRouter(<MissionControl />));
    await waitFor(() => expect(document.querySelector("[aria-live='polite']")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("keeps all Mission Control resources aligned across registered locales", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      for (const key of ["title", "attention", "sortAttention", "action.review", "facet.logs", "state.waiting_approval"]) {
        expect(i18n.getFixedT(locale)(`missionControl.${key}`)).not.toBe(`missionControl.${key}`);
      }
    }
  });

  it("renders an explicit empty state", async () => {
    const empty: MissionControlOverview = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(empty);
    render(withRouter(<MissionControl />)); await waitFor(() => expect(document.querySelector("[data-testid='mission-control'] .p-8")).toBeTruthy());
  });

  it("stops polling once unmounted", async () => {
    const empty: MissionControlOverview = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    const overview = vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(empty);
    const { unmount } = render(withRouter(<MissionControl />));
    await waitFor(() => expect(overview).toHaveBeenCalled());

    unmount();
    const callsAtUnmount = overview.mock.calls.length;
    // Longer than the 2s poll interval: proves the interval, not just this test's patience, stopped.
    await new Promise((resolve) => setTimeout(resolve, 2_500));
    expect(overview.mock.calls.length).toBe(callsAtUnmount);
  });

  // 16.14-16.15: use-mission-control-actions.ts's own per-run mutation registry, exercised through
  // the full page rather than a hook-isolated renderHook -- matching use-work-board-actions.ts's
  // own test coverage, which likewise only lives inside work-board.test.tsx.
  it("disables only the acting run's own buttons while its action is pending, leaving an unrelated run's buttons enabled", async () => {
    await activateAppLanguage("en");
    let resolvePause: ((receipt: unknown) => void) | undefined;
    vi.spyOn(agentService, "performMissionControlAction").mockImplementationOnce(
      () => new Promise((resolve) => { resolvePause = resolve as (receipt: unknown) => void; }),
    );

    render(withRouter(<MissionControl />));
    const pausedCard = await screen.findByTestId(`mission-run-${PAUSED_RUN_ID}`);
    const runningCard = await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);
    const resumeButton = within(pausedCard).getByRole("button", { name: "Resume" }) as HTMLButtonElement;
    const otherCancelButton = within(runningCard).getByRole("button", { name: "Cancel" }) as HTMLButtonElement;

    fireEvent.click(resumeButton);

    await waitFor(() => expect(resumeButton.disabled).toBe(true));
    expect(within(pausedCard).getByRole("status")).toBeTruthy();
    // Per-run, not page-wide: the unrelated run's own action stays enabled throughout.
    expect(otherCancelButton.disabled).toBe(false);

    const receipt = await webAgentClient.getMissionControlRun(PAUSED_RUN_ID);
    resolvePause?.({ run: { ...receipt.run, state: "running", version: receipt.run.version + 1, actions: ["open", "cancel"] }, operationId: null });

    await waitFor(() => expect(within(pausedCard).queryByRole("status")).toBeNull());
  });

  it("does not reload the whole board after a single run's own successful action", async () => {
    await activateAppLanguage("en");
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(withRouter(<MissionControl />));
    const pausedCard = await screen.findByTestId(`mission-run-${PAUSED_RUN_ID}`);
    await waitFor(() => expect(overview).toHaveBeenCalledOnce());
    const loadCallsAfterMount = overview.mock.calls.length;

    // Resume keeps the run within the same (active) section throughout, so the same DOM node can
    // be checked for its own updated content -- unlike cancel, which (correctly, per
    // mission-control-run-precedence.ts's own documented "never fabricate a section move" rule)
    // drops a run out of view once it turns terminal, until the next natural load().
    fireEvent.click(within(pausedCard).getByRole("button", { name: "Resume" }));

    await waitFor(() => expect(within(pausedCard).getByText("Running")).toBeTruthy());
    expect(overview.mock.calls.length).toBe(loadCallsAfterMount);
  });

  it("detects a real version conflict from the Web backend, refreshes the affected run, and explains it on that run alone", async () => {
    await activateAppLanguage("en");
    render(withRouter(<MissionControl />));
    const pausedCard = await screen.findByTestId(`mission-run-${PAUSED_RUN_ID}`);
    const runningCard = await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);
    await waitFor(() => expect(within(pausedCard).getByRole("button", { name: "Resume" })).toBeTruthy());

    // Simulate the run being resumed elsewhere (another window, another operation) after this page
    // already loaded it -- the rendered card still carries the pre-change version in its own
    // closure, so acting on it below goes through the real Web backend's own conflict guard
    // (web-mission-control-client.ts / updateWebAgentRun), not a mocked rejection. Resuming (rather
    // than cancelling) keeps the run in the same "active" section throughout, so this test can
    // check the one card's own content in place -- see the "no full reload" test above.
    updateWebAgentRun(PAUSED_RUN_ID, 4, "running");

    fireEvent.click(within(pausedCard).getByRole("button", { name: "Resume" }));

    await waitFor(() => expect(within(pausedCard).getByText("This Run changed elsewhere. It has been refreshed to the latest state.")).toBeTruthy());
    expect(within(pausedCard).getByText("Running")).toBeTruthy();
    expect(within(pausedCard).queryByRole("button", { name: "Resume" })).toBeNull();
    // Attributed to this one run -- an unrelated run's own card shows no such explanation.
    expect(within(runningCard).queryByText(/changed elsewhere/)).toBeNull();
  });

  // 16.4: metric cards as filters.
  it("applies the exact mapped filter when a metric card is clicked, including the two-state 'blocked' card, and toggles off on a second click", async () => {
    await activateAppLanguage("en");
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(withRouter(<MissionControl />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));

    const blockedCard = await screen.findByTestId("mission-control-count-blocked");
    expect(blockedCard.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(blockedCard);
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ states: ["blocked", "stuck"] })));
    expect(blockedCard.getAttribute("aria-pressed")).toBe("true");

    fireEvent.click(blockedCard);
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ states: undefined })));
    expect(blockedCard.getAttribute("aria-pressed")).toBe("false");
  });

  it("shows a single-state metric card as pressed when the same state is set manually from the status dropdown -- one shared filter, two entry points", async () => {
    await activateAppLanguage("en");
    render(withRouter(<MissionControl />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));

    openByTrigger(/Filters/);
    fireEvent.change(screen.getByLabelText("Filter by status"), { target: { value: "failed" } });

    const failedCard = await screen.findByTestId("mission-control-count-failed");
    await waitFor(() => expect(failedCard.getAttribute("aria-pressed")).toBe("true"));
  });

  // 16.5: Toolbar/FilterPopover migration preserves every existing filter's actual behavior.
  it("filters by exact Agent id via the new dropdown, by free-text project id, and keeps Sort working outside the popover", async () => {
    await activateAppLanguage("en");
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(withRouter(<MissionControl agents={[agentFixture("web-owner-6", "Test Agent Six")]} />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));

    openByTrigger(/Filters/);
    fireEvent.change(screen.getByLabelText("Filter by Agent"), { target: { value: "web-owner-6" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ agentId: "web-owner-6", cursor: null })));

    fireEvent.change(screen.getByLabelText("Filter by project ID"), { target: { value: "proj-1" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ projectId: "proj-1", cursor: null })));

    fireEvent.change(screen.getByLabelText("Sort Runs"), { target: { value: "newest" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ sort: "newest", cursor: null })));
  });

  // 16.6: Saved Views.
  it("saves the current filters under a name and reapplies them exactly on Apply", async () => {
    await activateAppLanguage("en");
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(withRouter(<MissionControl />));
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));

    openByTrigger(/Filters/);
    fireEvent.change(screen.getByLabelText("Filter by status"), { target: { value: "failed" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ states: ["failed"] })));

    openByTrigger("Saved views");
    fireEvent.change(screen.getByLabelText("View name"), { target: { value: "Only failed" } });
    fireEvent.click(screen.getByRole("button", { name: "Save current filters" }));
    expect(await screen.findByRole("button", { name: "Only failed" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ states: undefined })));

    fireEvent.click(screen.getByRole("button", { name: "Only failed" }));
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ states: ["failed"] })));
  });

  // 16.7: safe Agent/owner labels.
  it("resolves a matching Agent's own display name, translates a non-Agent owner type, and falls back to the raw id when no registry entry matches", async () => {
    await activateAppLanguage("en");
    const { rerender } = render(withRouter(<MissionControl />));
    const runningCard = await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);
    // No agents supplied yet: an honest fallback to the raw owner id, not a blank or a crash.
    // Scoped to the owner-label <p> specifically -- the run's own title ("Run web-owner-6", from
    // the Web mock's fixture data) also contains this id as a substring, so an unscoped query
    // would match two elements.
    expect(within(runningCard).getByText(/web-owner-6/, { selector: "p" })).toBeTruthy();

    const pausedCard = await screen.findByTestId(`mission-run-${PAUSED_RUN_ID}`);
    // ownerType "web_demo" carries no agentId at all (the Web mock only sets agentId when
    // ownerType is exactly "agent") -- translated the same way `reasonCode` already is, not shown
    // as the raw internal token.
    expect(within(pausedCard).getByText(/Web demo/, { selector: "p" })).toBeTruthy();

    rerender(withRouter(<MissionControl agents={[agentFixture("web-owner-6", "Test Agent Six")]} />));
    await waitFor(() => expect(within(screen.getByTestId(`mission-run-${RUNNING_RUN_ID}`)).getByText(/Test Agent Six/, { selector: "p" })).toBeTruthy());
  });

  // 20.3: below 900px the run list and the selected Run's own detail must never both render at
  // once under this component's `overflow-hidden` grid -- previously they did, and a reader with
  // more than a screenful of Runs could never reach the detail pane at all (a genuine "clipped
  // column", not an accessible fallback). Mirrors projects.test.tsx's own "compact layout" suite.
  describe("compact layout (20.3)", () => {
    beforeEach(() => stubMatchMedia((query) => query === "(max-width: 899px)"));
    afterEach(() => stubMatchMedia(() => false));

    it("shows only the list until a Run is selected, never the empty detail placeholder", async () => {
      await activateAppLanguage("en");
      render(withRouter(<MissionControl />));
      await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);

      expect(screen.queryByText("Select a Run to inspect available evidence.")).toBeNull();
      expect(screen.queryByRole("heading", { name: "Run detail" })).toBeNull();
    });

    it("inspecting a Run replaces the list with its detail and a Back control", async () => {
      await activateAppLanguage("en");
      render(withRouter(<MissionControl />));
      const card = await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);
      // Sanity check that the list really did have more than one Run in it -- otherwise the
      // "gone entirely" assertion below would trivially pass even if the list merely never
      // rendered a second copy of the same card the detail pane goes on to show.
      await screen.findByTestId(`mission-run-${PAUSED_RUN_ID}`);
      // A plain DOM query for the card's first button, not `getByRole`/`{ name }`: the inspect
      // button's accessible name is the whole card's rendered text (title, runner, state, owner,
      // elapsed, verification), not a short fixed label -- same reason the very first test in this
      // file (`document.querySelector("[data-testid^='mission-run-'] button")`) does the same.
      fireEvent.click(card.querySelector("button")!);

      await waitFor(() => expect(screen.getByRole("heading", { name: "Run detail" })).toBeTruthy());
      // Compact never renders both panes at once. Checked against a *different*, non-selected
      // Run's own card, not the selected Run's own testid: `MissionControlDetailPanel` renders its
      // own copy of the selected Run's `RunCard` (same `data-testid`) as part of the detail itself,
      // so that id staying present is expected, not proof the list is still rendered underneath.
      expect(screen.queryByTestId(`mission-run-${PAUSED_RUN_ID}`)).toBeNull();
      expect(screen.getByRole("button", { name: "Back to Runs" })).toBeTruthy();
    });

    it("Back returns to the list and drops the selected Run's detail", async () => {
      await activateAppLanguage("en");
      render(withRouter(<MissionControl />));
      const card = await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`);
      fireEvent.click(card.querySelector("button")!);
      await screen.findByRole("heading", { name: "Run detail" });

      fireEvent.click(screen.getByRole("button", { name: "Back to Runs" }));

      expect(await screen.findByTestId(`mission-run-${RUNNING_RUN_ID}`)).toBeTruthy();
      expect(screen.queryByRole("heading", { name: "Run detail" })).toBeNull();
    });
  });
});
