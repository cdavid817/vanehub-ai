import { useTranslation } from "react-i18next";
import { withReturnTo, workbenchPath } from "../main-layout/workbench-route";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlAction, MissionControlRunSummary } from "../types/mission-control";
import { EvidenceLink } from "../ui/evidence/EvidenceLink";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import { resolveAgentDisplayName } from "./mission-control-labels";

export interface RunCardProps {
  agents: readonly AgentRegistryEntry[];
  mutation?: MutationState;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onDismissError: (run: MissionControlRunSummary) => void;
  onInspect: (run: MissionControlRunSummary) => void;
  run: MissionControlRunSummary;
  /** 16.13: which Runs route tab this card is currently shown under (list row or the open detail
   *  view) -- the one thing needed, alongside `run.runId` (always known), to build a real, safe
   *  `returnTo` for the "review" action's own EvidenceLink. Left `undefined` in every test that has
   *  no real route tab to scope by (pre-existing precedent -- see `MissionControlRunListSection`'s
   *  own doc comment); the review EvidenceLink simply omits `returnTo` in that case rather than
   *  guessing one, matching `navigateFromMissionControl`'s own established "only build a return
   *  location when the section is one of the three known ones" precedent (main-layout.tsx). */
  section?: "attention" | "active" | "history";
}

/**
 * 16.7: prefers the Agent registry's own display name over the raw id, falling back to the raw id
 * (never blank, never the registry silently hiding a mismatch) when no entry matches -- see
 * mission-control-labels.ts's own doc comment for why a mismatch is a real, expected case and not
 * just defensive coding.
 *
 * When a run carries no `agentId` at all (an owner type the backend does not classify as Agent- or
 * generation-adjacent -- e.g. a bare "session", "loop_run", or the Web adapter's own "web_demo"
 * demo-seed owner), the raw `ownerType` itself is translated the same way `reasonCode` a few lines
 * below already is: a known-value lookup with a same-string fallback, never a raw internal token or
 * an untranslated i18n key leaking through.
 */
function runOwnerLabel(run: MissionControlRunSummary, agents: readonly AgentRegistryEntry[], t: (key: string, options?: Record<string, unknown>) => string): string {
  const agentLabel = resolveAgentDisplayName(agents, run.agentId);
  if (agentLabel) return agentLabel;
  return t(`missionControl.owner.${run.ownerType}`, { defaultValue: run.ownerType });
}

/**
 * 16.13: the "review" action is pure client-side navigation (`use-mission-control-actions.ts`'s
 * own doc comment), never a mutation -- so unlike the other action buttons in this same row, it is
 * a genuine `EvidenceLink` candidate rather than an `onAct` trigger. `availability` is derived from
 * a real signal, not assumed: both backends only ever set `run.navigation.kind === "review"` when a
 * review is actually linked (confirmed directly in `project()` / `webMissionSummary()`), but
 * `sessionId` on that target is itself optional (`session?.linkId` -- a review can exist without a
 * resolvable session link), so `"available"` is only claimed once a real session id is in hand to
 * link to. The target session itself is always a normal, listable one (reviews only ever attach to
 * `session_generation`-owned runs, confirmed by reading `start_canonical_loop`'s links, which never
 * include a `session` entry) -- unlike Loop Center's worker/verifier role sessions (16.13's own
 * evidence in tasks.md), so a plain route link here does not hit `useWorkspaceSessionRoute`'s
 * "unknown session" bounce-back.
 */
function reviewEvidence(run: MissionControlRunSummary, section: RunCardProps["section"], t: (key: string, options?: Record<string, unknown>) => string) {
  const target = run.navigation?.kind === "review" ? run.navigation : null;
  const sessionId = target?.sessionId ?? null;
  const currentLocation = section ? ({ destination: "runs", section, runId: run.runId } as const) : null;
  const returnTo = currentLocation ? { label: t("missionControl.title"), path: workbenchPath(currentLocation) } : undefined;
  if (!sessionId) {
    return { availability: "unavailable" as const, copyValue: target?.id, reason: t("missionControl.review.sessionUnavailable"), returnTo, to: "" };
  }
  const sessionPath = workbenchPath({ destination: "sessions", sessionId, creatingSession: false });
  // Reuses the same real, already-validated `?returnTo=` token the pre-existing
  // `navigateFromMissionControl` mechanism builds for this exact action (main-layout.tsx) -- so this
  // EvidenceLink does not regress the working "return to evidence source" affordance a bare `<Link>`
  // would otherwise silently drop.
  return { availability: "available" as const, copyValue: target?.id, reason: undefined, returnTo, to: currentLocation ? withReturnTo(sessionPath, currentLocation) : sessionPath };
}

export function RunCard({ agents, mutation, onAct, onDismissError, onInspect, run, section }: RunCardProps) {
  const { t } = useTranslation();
  const pending = mutation?.pending ?? false;
  const ended = run.endedAt ?? run.updatedAt;
  const elapsed = Math.max(0, Date.parse(ended) - Date.parse(run.createdAt));
  const review = reviewEvidence(run, section, t);
  return (
    <article className="rounded-md border border-border bg-card p-3" data-testid={`mission-run-${run.runId}`}>
      <button className="w-full text-left focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => void onInspect(run)} type="button">
        <div className="flex flex-wrap items-center gap-2">
          <span className="min-w-0 flex-1 truncate text-sm font-medium">{run.title}</span>
          {run.runner ? (
            <span className="inline-flex max-w-44 items-center gap-1 rounded border border-primary/30 bg-primary/5 px-1.5 py-0.5 text-[11px] text-primary" data-runner={run.runner.kind}>
              <span>{t(`runner.kind.${run.runner.kind}`)}</span>
              {/* 20.16: `hostLabel` is resolved from the real SSH target, not app-authored -- wrapped
                  in `<bdi>` so a strong-RTL or mixed-script host label cannot read the "· "
                  separator (or this badge's own state-badge neighbor) out of order. */}
              {run.runner.hostLabel ? <span className="truncate text-muted-foreground">· <bdi>{run.runner.hostLabel}</bdi></span> : null}
            </span>
          ) : null}
          <span className="rounded border border-border px-1.5 py-0.5 text-[11px]">{t(`missionControl.state.${run.state}`)}</span>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          {runOwnerLabel(run, agents, t)} · {t("missionControl.elapsed", { seconds: Math.round(elapsed / 1000) })} · {t(`missionControl.verification.${run.verification}`)}
        </p>
        {run.reasonCode ? <p className="mt-1 text-xs text-warning">{t(`runner.reason.${run.reasonCode}`, { defaultValue: run.reasonCode })}</p> : null}
      </button>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <div className="flex flex-wrap items-center gap-1">
          {run.actions.map((action) => (
            action === "review" ? (
              <EvidenceLink
                availability={review.availability}
                className="text-xs"
                copyValue={review.copyValue}
                key={action}
                label={t("missionControl.action.review")}
                reason={review.reason}
                returnTo={review.returnTo}
                to={review.to}
              />
            ) : (
              <button
                className="rounded-md border border-input px-2 py-1 text-xs hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
                data-action={action}
                disabled={pending}
                key={action}
                onClick={() => void onAct(run, action)}
                type="button"
              >
                {t(`missionControl.action.${action}`)}
              </button>
            )
          ))}
        </div>
        <MutationStatus onDismiss={() => onDismissError(run)} state={mutation} />
      </div>
    </article>
  );
}
