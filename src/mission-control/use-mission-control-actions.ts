import { useCallback, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { DisplayableError } from "../ui/async/async-view-state";
import { useMutationRegistry } from "../ui/async/mutation-state";
import type {
  MissionControlAction,
  MissionControlNavigationTarget,
  MissionControlOverview,
  MissionControlRunDetail,
  MissionControlRunSummary,
} from "../types/mission-control";
import { patchMissionControlRun } from "./mission-control-run-precedence";

function toDisplayableError(reason: unknown): DisplayableError {
  // Matches use-work-board-actions.ts's own note: redoing the same action from the same button,
  // against the run's own now-current state, already is the retry -- there is no separately cached
  // "last attempted value" a dedicated retry affordance would need to replay.
  return { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: false };
}

/**
 * Detects the `"run version conflict"` thrown by both backends' `performMissionControlAction` --
 * web-mission-control-client.ts throws it verbatim, and Rust's `ApplicationError::Conflict`
 * (src-tauri/src/contexts/operations/application/error.rs) serializes to that exact same literal
 * across the Tauri boundary (`CommandError`'s `Serialize` impl emits only its `message` field, never
 * a wrapping object with a `category` -- confirmed by reading it). Unlike Loop Center's own
 * `isLoopVersionConflict` (loop-definition-dialog.tsx), this needs neither an i18n lookup nor a
 * substring match: both backends already agree on one exact, untranslated string, so an exact match
 * is the more precise option available here. The structured `MissionControlSafeError`/
 * `code: "conflict"` type (types/mission-control.ts) is deliberately not used for detection:
 * grepping every service file for it turns up only its declaration, never a construction or a
 * catch-site use, and `CommandError` confirms Rust does not serialize it either -- detection has to
 * match what actually crosses the wire today, not the richer shape that was designed but never
 * wired up.
 */
export function isMissionControlVersionConflict(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message === "run version conflict";
}

export interface UseMissionControlActionsOptions {
  onNavigate?: (target: MissionControlNavigationTarget, sourceRunId: string) => void;
  setOverview: Dispatch<SetStateAction<MissionControlOverview | null>>;
  setSelected: Dispatch<SetStateAction<MissionControlRunDetail | null>>;
}

/**
 * Per-run mutation orchestration for Mission Control (tasks 16.14-16.15), replacing the previous
 * page-wide `error` plus an unconditional full-board `load()` after every action (mission-control
 * .tsx's old `act()`) with `useMutationRegistry` (src/ui/async/mutation-state.ts, §3.14) keyed by
 * `runId` -- the same registry `use-work-board-actions.ts` adopted, and for the same reason:
 * Mission Control's Attention/Active/Recent sections each render many `RunCard`s at once, each with
 * its own independent in-flight action, much closer to Work Board's "many simultaneously-visible
 * independent targets" shape than to Loop Center's single-detail-view one (`loop-run-controls.tsx`
 * uses one bare `pending` field because only one Loop run's controls are ever on screen there).
 *
 * Every action is reconcile-only, no optimistic guess -- matching Goal Center's own discipline
 * (`use-goal-center-actions.ts`), and for the same class of reason: `MissionControlRunSummary`
 * carries `state`, `attention`, `actions`, and `verification`, all recomputed server-side from the
 * run's own state machine as a side effect of any action (see `project()` in
 * `contexts/operations/application/mission_control.rs` / `webMissionSummary` in
 * web-mission-control-client.ts) -- a client-side guess at the post-cancel or post-resume `actions`
 * array would just be re-deriving that same server logic a second time, with no way to keep the
 * copy honest. `open`/`approval`/`review` never reach the network at all -- they are pure
 * client-side navigation, matching the original `act()`'s own behavior -- so they carry no
 * mutation state.
 */
export function useMissionControlActions({ onNavigate, setOverview, setSelected }: UseMissionControlActionsOptions) {
  const { t } = useTranslation();
  const mutations = useMutationRegistry();

  const applyRun = useCallback((fresh: MissionControlRunSummary) => {
    setOverview((current) => (current ? patchMissionControlRun(current, fresh) : current));
    setSelected((current) => (current && current.run.runId === fresh.runId ? { ...current, run: fresh } : current));
  }, [setOverview, setSelected]);

  /**
   * design.md Decision 14's own "冲突时刷新 canonical state 并解释" (on conflict, refresh canonical
   * state and explain), applied the way `loop-definition-dialog.tsx`'s own `handleVersionConflict`
   * applies it for Loop definitions: refetch the one affected run (Mission Control has a real
   * single-run getter, `getMissionControlRun`, unlike Loop Center's definition-list-only
   * situation), patch every local view of it, and attribute a specific, translated explanation to
   * that run's own mutation state rather than a page-wide error.
   */
  const reconcileConflict = useCallback(async (runId: string) => {
    try {
      const fresh = await agentService.getMissionControlRun(runId);
      applyRun(fresh.run);
      mutations.fail(runId, { kind: "error", message: t("missionControl.actionConflict"), retryable: false });
    } catch {
      mutations.fail(runId, { kind: "error", message: t("missionControl.actionConflictRemoved"), retryable: false });
    }
  }, [applyRun, mutations, t]);

  const act = useCallback(async (run: MissionControlRunSummary, action: MissionControlAction) => {
    if (action === "open" || action === "approval" || action === "review") {
      if (run.navigation) onNavigate?.(run.navigation, run.runId);
      return;
    }
    // Defensive: RunCard already disables a run's own action buttons while its mutation is
    // pending, so in practice this only guards a caller that bypasses that UI gate.
    if (mutations.get(run.runId)?.pending) return;
    mutations.begin(run.runId);
    try {
      const receipt = await agentService.performMissionControlAction({ runId: run.runId, version: run.version, action });
      applyRun(receipt.run);
      mutations.succeed(run.runId);
    } catch (reason: unknown) {
      if (isMissionControlVersionConflict(reason)) { await reconcileConflict(run.runId); return; }
      mutations.fail(run.runId, toDisplayableError(reason));
    }
  }, [applyRun, mutations, onNavigate, reconcileConflict]);

  return { act, mutations };
}
