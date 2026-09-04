// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useEvaluationQuery } from "../evaluation-center/use-evaluation-query";
import { useContainerCompactMode } from "../hooks/use-container-compact-mode";
import { useLoopElapsed } from "../loop-center/loop-monitoring";
import { PlanDestination } from "../main-layout/plan-destination";
import { ProjectsDestination } from "../main-layout/projects-destination";
import { useSessionStreamEvents } from "../main-layout/use-session-stream-events";
import { useMissionControlPolling } from "../mission-control/use-mission-control-polling";
import { subscribeLoopRunPolling } from "../services/loop-run-polling";
import { SettingsShell } from "../settings/settings-shell";
import { renderWithAppProviders } from "../test/render";
import type { EvaluationArena } from "../types/evaluation";
import type { LoopRun } from "../types/loop";
import { createResourceRegistry, createTrackedResizeObserver, getActivePendingTimerCount } from "./resource-tracking";

/**
 * Task 21.22: "run memory/leak navigation loops across Sessions, Runs, Plan, Quality, Projects, and
 * Settings and compare mounted resources to baseline." These are the six *top-level* destinations
 * (`workbench-route.ts`'s `WorkbenchDestination` plus Settings, App.tsx's own separate
 * `<Route path="/settings">` sibling of `<Route path="/workspace/*">`) -- not the finer-grained
 * sub-features 21.16 already proved a single mount/unmount pair releases cleanly for. This file's
 * own distinct value, per this task's own framing, is proving that property holds under *repeated*
 * cycling (3-5+ full loops), which a single mount/unmount pair cannot distinguish from a slow
 * accumulation that only shows up after N navigations (e.g. a release path that only decrements
 * correctly the first time).
 *
 * **Shape chosen, and why**: a full top-level destination is a large component tree wired to many
 * real services (`useMainLayoutModel` alone needs 11+ `agentService` methods plus
 * `permissionsService`/`settingsService`, confirmed by reading `use-main-layout-recovery.test.tsx`'s
 * own mock setup) -- mounting six of those repeatedly in one test would be slow and would drown the
 * actual resource-lifecycle proof in incidental service-mocking noise. Per this task's own explicit
 * allowance, this file instead targets the real resource-owning primitives directly (the same hooks
 * 21.16 already proved release once), repeated in a loop, for the three destinations that 21.16
 * confirmed actually own a page-level interval/observer/subscription resource (Sessions, Runs,
 * Quality). For the three that 21.16 (Plan, Projects) or this task's own fresh re-verification
 * (Settings, below) confirmed own *no* such resource, there is nothing to assert a resource count on
 * -- per this task's own instruction not to fabricate one -- so those instead get the lighter, still
 * real proof that their own top-level destination component mounts and unmounts cleanly (no thrown
 * error) across the same repeat count.
 *
 * **Re-verification done before writing this file** (not just trusting 21.16's own summary):
 * re-grepped every domain directory 21.16 covered (mission-control, loop-center, evaluation-center,
 * work-board, goal-center, scheduled-tasks, projects) for the same interval/observer/subscription API
 * list 21.16 used. Confirmed unchanged: Work Board/Goal Center/Scheduled Tasks/Projects still have
 * zero production matches (only their own `.test.*` files, or -- work-board/mission-control's own
 * saved-view-menu and Loop Center's preflight/definition dialogs -- the same open-scoped
 * `pointerdown`/`keydown`-while-a-transient-element-is-open listeners 21.16 already found and
 * correctly left alone). Also confirmed `event-coalescer.ts`'s `createMissionControlCoalescer` is
 * still dead code with no live producer (16.16's own doc comment on `use-mission-control-polling.ts`),
 * not a second Mission Control resource. Newly checked here, since Settings was outside 21.16's own
 * 7-destination scope: `settings-shell.tsx`/`settings-topbar.tsx`/`settings-sidebar.tsx`/
 * `settings-compact-nav.tsx`/`settings-search-box.tsx`/`settings-provider.tsx` (the shell chrome
 * itself, i.e. what is mounted regardless of which of the ~20 individual settings pages is active)
 * have zero matches either. `use-settings-anchor-highlight.ts` does poll (`window.setTimeout`), but
 * only while `anchorId` is non-null -- i.e. only while a search-result field highlight is actively
 * resolving, an interaction-scoped trigger matching the same "self-contained, already correctly
 * scoped, not a page-mount-lifetime resource" category 21.16 already established for Work Board's own
 * open-scoped listener, not something a bare Settings mount/unmount ever arms.
 *
 * **A real gap this file closes**: `use-session-stream-events.ts` (Sessions' own message-streaming
 * subscription) was 21.16's own disclosed-but-not-closed gap ("Sessions is not one of this task's 7
 * named 'ordinary destinations' ... Left as a disclosed gap for whichever future task owns Sessions'
 * own resource lifecycle"). This task names Sessions directly, and per `destination-lifecycle.ts`
 * (`sessions: { keepAlive: "draft-only" }`) plus `main-layout.tsx`'s own CSS-hidden-not-unmounted
 * rendering, Sessions never actually unmounts on an ordinary within-`/workspace` destination switch
 * -- the *only* real trigger is leaving `/workspace` entirely, which `App.tsx`'s `<Routes>` only does
 * for a Settings visit (a plain sibling `<Route>`, unmounted/remounted by React Router itself on every
 * switch). That is exactly this task's own named Sessions<->Settings boundary, so it is now in scope
 * and closed below.
 *
 * **A second, real gap found here, disclosed but deliberately not closed in this pass**:
 * `use-main-layout-model.ts` also holds a second, *separate* subscription with the same
 * whole-`MainLayout`-lifetime boundary -- `permissionsService.subscribePendingApprovalEvents`
 * (a global pending-approval notifier, cleaned up via the same "leaving /workspace" unmount).
 * `use-main-layout-recovery.test.tsx` already mocks it (`vi.fn().mockResolvedValue(vi.fn())`) but
 * never asserts the unsubscribe actually fires, and it can only be exercised through the full
 * `useMainLayoutModel()` hook (11+ mocked `agentService` methods, real `waitFor`s on real timers,
 * confirmed by reading that file) -- not a hook-boundary-isolated primitive the way
 * `useSessionStreamEvents` is. Repeating that full setup 3-5x here for one additional subscription
 * count would cost far more than it proves beyond what this file's own Sessions case (identical
 * unmount boundary) already demonstrates the *mechanism* is sound for. Left as a disclosed, real,
 * out-of-scope-here gap for the same reason 21.16 left Work Board's listener alone: effort/value.
 */

vi.mock("../components/lazy-feature", () => ({ LazyFeature: () => <div data-testid="lazy-feature-stub" /> }));

const service = vi.hoisted(() => ({
  listAgents: vi.fn(),
  listEvaluationArenas: vi.fn(),
  listEvaluationTasks: vi.fn(),
  subscribeMessageEvents: vi.fn(),
}));
vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));

function loopRun(updatedAt: string): LoopRun {
  return { id: "run-1", updatedAt } as LoopRun;
}

function evaluationArena(outcome: EvaluationArena["attempts"][number]["outcome"]): EvaluationArena {
  return {
    id: "arena-1", operationId: "operation-1", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2",
    attempts: [{
      id: "attempt-1", arenaId: "arena-1", canonicalRunId: "run-1", taskId: "fix-null-auth-token", taskVersion: 1,
      agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" },
      outcome, checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
    }],
  };
}

const CYCLES = 5;

describe("21.22 navigation loop -- real page-owned resources (Sessions/Runs/Quality)", () => {
  beforeEach(() => vi.unstubAllGlobals());
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it(`returns every tracked resource to the same zero baseline after each of ${CYCLES} full Sessions-Runs-Plan-Quality-Projects-Settings cycles`, async () => {
    vi.useFakeTimers();
    service.listAgents.mockResolvedValue([]);
    service.listEvaluationTasks.mockResolvedValue([]);
    service.listEvaluationArenas.mockResolvedValue({ items: [evaluationArena("queued")], nextCursor: null });

    const tracked = createTrackedResizeObserver();
    vi.stubGlobal("ResizeObserver", tracked.Ctor);
    const sessionRegistry = createResourceRegistry();
    service.subscribeMessageEvents.mockImplementation(async () => {
      const id = sessionRegistry.acquire();
      return () => sessionRegistry.release(id);
    });

    const container = document.createElement("div");
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    const loadRun = vi.fn<() => Promise<LoopRun>>().mockResolvedValue(loopRun("2026-07-22T10:00:00Z"));
    const runningRun = { status: "running", startedAt: "2026-07-23T00:00:00Z", createdAt: "2026-07-23T00:00:00Z" } as LoopRun;

    // Baseline before the first cycle even starts -- proves the zero every cycle returns to below is
    // a genuine idle state, not a coincidental repeat of some pre-existing count.
    expect(getActivePendingTimerCount()).toBe(0);
    expect(tracked.activeCount()).toBe(0);
    expect(sessionRegistry.activeCount()).toBe(0);

    for (let cycle = 0; cycle < CYCLES; cycle += 1) {
      // -- Sessions -- destination-lifecycle.ts: `keepAlive: "draft-only"`, CSS-hidden only, so this
      // is the one destination that does NOT unmount across the Runs/Plan/Quality/Projects steps
      // below. It mounts here, at the top of the cycle (arriving back from Settings, or the app's own
      // first load), and only unmounts at the Settings step at the bottom of this same cycle.
      const sessions = renderHook(() => useSessionStreamEvents({
        invalidateSessions: vi.fn(), messagesKey: ["messages", "session-1", 50] as const, onTurnStatus: vi.fn(), sessionId: "session-1",
      }), { wrapper });
      // Flushes the subscription's own async `.then()` -- fake timers do not mock microtask
      // scheduling, only setInterval/setTimeout (same technique use-evaluation-query.test.ts's own
      // 21.16 case already established for an identical async-initial-load shape).
      await act(async () => { await vi.advanceTimersByTimeAsync(0); });
      expect(sessionRegistry.activeCount()).toBe(1);

      // -- Runs: Mission Control (attention/active/history) -- the more frequent of Runs' two real
      // resources per 21.16 (it unmounts on every intra-Runs tab switch too, not only a full exit).
      const missionControl = renderHook(() => useMissionControlPolling(reconcile));
      const missionControlNav = renderHook(() => useContainerCompactMode({ current: container }, 640));
      expect(getActivePendingTimerCount()).toBe(1);
      expect(tracked.activeCount()).toBe(1);
      missionControl.unmount();
      missionControlNav.unmount();
      expect(getActivePendingTimerCount()).toBe(0);
      expect(tracked.activeCount()).toBe(0);

      // -- Runs: Loop Center (loops tab) -- a separate, mutually exclusive visit to Runs' own other
      // real resource pair (runs-destination.tsx only ever mounts one of Mission
      // Control/Loops/Schedules at a time). Runs' third tab, Schedules, is confirmed to own no
      // resource at all (re-verified above), so it is not modeled as a step here.
      const unsubscribeLoopPoll = subscribeLoopRunPolling(loadRun, vi.fn(), 100);
      const loopElapsed = renderHook(() => useLoopElapsed(runningRun));
      // subscribeLoopRunPolling's own setInterval + useLoopElapsed's own setInterval.
      expect(getActivePendingTimerCount()).toBe(2);
      unsubscribeLoopPoll();
      loopElapsed.unmount();
      expect(getActivePendingTimerCount()).toBe(0);

      // -- Plan (Work Board / Goal Center) -- confirmed, fresh, above: no page-owned resource, so
      // nothing mounts here. See the separate "component tree mounts cleanly" check below.

      // -- Quality (Evaluation) --
      const evaluationQuery = renderHook(() => useEvaluationQuery());
      await act(async () => { await vi.advanceTimersByTimeAsync(0); });
      const evaluationTable = renderHook(() => useContainerCompactMode({ current: container }, 640));
      expect(getActivePendingTimerCount()).toBe(1);
      expect(tracked.activeCount()).toBe(1);
      evaluationQuery.unmount();
      evaluationTable.unmount();
      expect(getActivePendingTimerCount()).toBe(0);
      expect(tracked.activeCount()).toBe(0);

      // -- Projects -- confirmed no page-owned resource either; nothing mounts here, see below.

      // -- Settings -- the one real boundary that unmounts the entire /workspace route (App.tsx's
      // sibling <Routes> entries), including Sessions' own MainLayout subtree -- this is where
      // Sessions' own subscription (mounted at the top of this same cycle) actually releases.
      sessions.unmount();
      expect(sessionRegistry.activeCount()).toBe(0);

      // End of cycle: every resource this cycle touched is back to zero -- asserted per resource
      // type, not pooled into one summed number, because a blended total could mask exactly the bug
      // this task cares about (e.g. Mission Control leaking +1 while Loop Center's own count happens
      // to read -1 that same cycle would net to zero and hide both).
      expect(getActivePendingTimerCount()).toBe(0);
      expect(tracked.activeCount()).toBe(0);
      expect(sessionRegistry.activeCount()).toBe(0);
    }
  });
});

describe("21.22 navigation loop -- Plan/Projects/Settings own no page-level resource, so their own proof is a clean repeated mount/unmount", () => {
  it(`PlanDestination (Work Board + Goal Center tabs) mounts and unmounts cleanly across ${CYCLES} repeated cycles`, () => {
    for (let cycle = 0; cycle < CYCLES; cycle += 1) {
      const { unmount } = render(<PlanDestination location={{ section: "board" }} onSectionChange={vi.fn()} />);
      unmount();
    }
  });

  it(`ProjectsDestination mounts and unmounts cleanly across ${CYCLES} repeated cycles`, () => {
    for (let cycle = 0; cycle < CYCLES; cycle += 1) {
      const { unmount } = render(
        <ProjectsDestination onContinueSession={vi.fn()} onNewSessionForWorkspace={vi.fn()} onOpenSettings={vi.fn()} />,
      );
      unmount();
    }
  });

  it(`SettingsShell mounts and unmounts cleanly across ${CYCLES} repeated cycles`, () => {
    for (let cycle = 0; cycle < CYCLES; cycle += 1) {
      const { unmount } = renderWithAppProviders(<MemoryRouter><SettingsShell onReturn={vi.fn()} /></MemoryRouter>);
      unmount();
    }
  });
});
