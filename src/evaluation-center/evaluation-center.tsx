import { useEffect, useMemo, useState } from "react";
import { ShieldCheck, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena, EvaluationAttempt } from "../types/evaluation";
import { StatusBadge } from "../ui/status/StatusBadge";
import { EvidenceLink } from "../ui/evidence/EvidenceLink";
import { EvaluationArenaList } from "./evaluation-arena-list";
import { OUTCOME_TONE } from "./evaluation-arena-summary";
import { EvaluationComparisonPanel } from "./evaluation-comparison-panel";
import { EvaluationResultsTable } from "./evaluation-results-table";
import { EvaluationRunControls } from "./evaluation-run-controls";
import { TERMINAL_EVALUATION_OUTCOMES, useEvaluationQuery } from "./use-evaluation-query";

export function EvaluationCenter() {
  const { t } = useTranslation();
  const { agents, tasks, arenas, setArenas, error, setError } = useEvaluationQuery();
  const [taskId, setTaskId] = useState("");
  const [agentIds, setAgentIds] = useState<string[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [running, setRunning] = useState(false);
  // Seeds the task/Agent selection from whatever useEvaluationQuery last fetched. Runs once at
  // mount (against the hook's empty initial state -- a no-op) and again whenever a real fetch
  // lands, reproducing the single combined effect this used to be before the fetch itself moved
  // into the hook: every fetch (including a re-fetch) still resets the selection to its default.
  useEffect(() => { setTaskId(tasks[0]?.id ?? ""); }, [tasks]);
  useEffect(() => {
    const available = agents.filter((agent) => agent.availabilityState === "available");
    setAgentIds((available.length > 0 ? available : agents).map((agent) => agent.id));
  }, [agents]);
  // Derived rather than held: an attempt captured at click time is a snapshot of a run still in
  // flight, and the polling below replaces the arena it came from without ever touching it. The
  // detail pane went on showing `queued` -- Cancel button and all -- beside a row that had already
  // reported its verdict.
  const selected = useMemo(
    () => arenas.flatMap((arena) => arena.attempts).find((attempt) => attempt.id === selectedId) ?? null,
    [arenas, selectedId],
  );
  // 18.12 "thresholds": the only real threshold-like value anywhere in the evaluation domain is
  // `EvaluationTask.timeoutSeconds` -- `EvaluationCheck`/`EvaluationMetric` carry no target/threshold
  // field of their own (checked both types fully), so this is a task-level lookup, not per-check or
  // per-metric. `null` (never a fabricated "Unavailable" line) when the catalog has no matching
  // task/version, mirroring `findTaskPrompt`'s own established "omit rather than fabricate" choice.
  const selectedTask = useMemo(
    () => (selected ? tasks.find((task) => task.id === selected.taskId && task.version === selected.taskVersion) ?? null : null),
    [tasks, selected],
  );
  const visible = useMemo(() => arenas.flatMap((arena) => arena.attempts.map((attempt) => ({ arena, attempt })))
    .filter(({ attempt }) => `${attempt.agent.agentId} ${attempt.outcome}`.toLowerCase().includes(filter.toLowerCase())), [arenas, filter]);
  // Independent of `visible`/`filter` on purpose (18.8): comparing two results is not scoped to
  // whatever text the results table happens to be filtered to.
  const allAttempts = useMemo(() => arenas.flatMap((arena) => arena.attempts), [arenas]);
  // Takes the wizard's own draft values as parameters rather than reading page state, so there is
  // no stale-closure race against the `setTaskId`/`setAgentIds` commit below (state updates are
  // not synchronous, and `EvaluationRunControls` calls this the instant Review's Run is clicked).
  async function start(nextTaskId: string, nextAgentIds: string[]): Promise<boolean> {
    const task = tasks.find((item) => item.id === nextTaskId);
    if (!task || nextAgentIds.length === 0) return false;
    setTaskId(nextTaskId); setAgentIds(nextAgentIds);
    setRunning(true); setError(null);
    try {
      const arena = await agentService.startEvaluation({ taskId: task.id, taskVersion: task.version, agentIds: nextAgentIds });
      setArenas((items) => [arena, ...items]); setSelectedId(arena.attempts[0]?.id ?? null);
      return true;
    } catch { setError(t("evaluation.runError")); return false; } finally { setRunning(false); }
  }
  async function cancel() {
    if (!selected) return;
    try { replaceArena(await agentService.cancelEvaluation(selected.arenaId)); }
    catch { setError(t("evaluation.cancelError")); }
  }
  function replaceArena(next: EvaluationArena) { setArenas((items) => items.map((item) => item.id === next.id ? next : item)); }
  async function exportArena(arena: EvaluationArena) {
    const payload = await agentService.exportEvaluation(arena.id);
    const link = document.createElement("a"); link.href = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }));
    link.download = `${arena.id}.json`; link.click(); URL.revokeObjectURL(link.href);
  }
  return <div className="ucd-panel flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg" data-testid="evaluation-center">
    <header className="flex flex-wrap items-center gap-3 border-b border-border p-3">
      <div className="min-w-48 flex-1"><h1 className="text-sm font-semibold">{t("evaluation.title")}</h1><p className="text-xs text-muted-foreground">{t("evaluation.description")}</p></div>
      <EvaluationRunControls
        agentIds={agentIds}
        agents={agents}
        error={error}
        onOpen={() => setError(null)}
        onRun={start}
        running={running}
        taskId={taskId}
        tasks={tasks}
      />
    </header>
    {error ? <p className="border-b border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive" role="alert">{error}</p> : null}
    <EvaluationArenaList arenas={arenas} tasks={tasks} />
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-auto lg:grid-cols-[minmax(420px,1.3fr)_minmax(280px,0.7fr)]">
      <EvaluationResultsTable
        filter={filter}
        onExportArena={(arena) => { void exportArena(arena); }}
        onFilterChange={setFilter}
        onSelectAttempt={setSelectedId}
        rows={visible}
      />
      <aside className="min-w-0 p-3" data-selected-attempt={selected?.id ?? ""} data-selected-outcome={selected?.outcome ?? ""} data-testid="evaluation-detail"><div className="mb-2 flex items-center justify-between"><h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.detail")}</h2>{selected && !TERMINAL_EVALUATION_OUTCOMES.has(selected.outcome) ? <button className="flex items-center gap-1 rounded-md border border-input px-2 py-1 text-xs" data-testid="evaluation-cancel" onClick={() => void cancel()} type="button"><Square className="h-3 w-3" />{t("evaluation.cancel")}</button> : null}</div>
        {selected ? <div className="space-y-3"><div className="rounded-md border border-border bg-muted/30 p-3"><p className="font-mono text-xs">{selected.agent.providerId} / {selected.agent.modelId ?? t("evaluation.unavailable")}</p><p className="mt-1 text-xs text-muted-foreground">{selected.agent.configurationFingerprint}</p></div>
          {/* 18.12: failure classification -- which one of the 9 EvaluationOutcome values this
             attempt has, plus a real per-outcome explanation (not a generic pass/fail badge), a
             task-level threshold when one exists, and an honest disclosure that no judge role is
             ever recorded (the `judge` field is structurally optional but has zero real producers
             or consumers anywhere in this codebase -- see evaluation.ts and the grep this task's
             own evidence in tasks.md cites). */}
          <div className="rounded-md border border-border bg-muted/30 p-3" data-testid="evaluation-outcome-detail">
            <StatusBadge label={t(`evaluation.outcome.${selected.outcome}`)} tone={OUTCOME_TONE[selected.outcome]} />
            <p className="mt-1 text-xs text-muted-foreground">{t(`evaluation.outcomeExplanation.${selected.outcome}`)}</p>
            {selectedTask ? <p className="mt-1 text-xs text-muted-foreground">{t("evaluation.timeoutLabel")}: {t("evaluation.timeoutValue", { seconds: selectedTask.timeoutSeconds })}</p> : null}
            <p className="mt-1 text-xs text-muted-foreground">{t("evaluation.judgeUnavailable")}</p>
          </div>
          <Evidence attempt={selected} title={t("evaluation.verification")} />
          <div>
            <h3 className="text-xs font-semibold">{t("evaluation.diff")}</h3>
            {selected.artifactIds.length === 0
              ? <p className="mt-1 text-xs text-muted-foreground">{t("evaluation.unavailable")}</p>
              : <ul className="mt-1 flex max-h-32 flex-col gap-1 overflow-auto">
                {selected.artifactIds.slice(0, 20).map((artifactId) => (
                  // No artifact-preview navigation target exists anywhere yet (18.13) — `unavailable`
                  // is the honest state, not a fabricated link to nowhere. `to` is inert on this
                  // branch: EvidenceLink never renders the `Link` element while unavailable.
                  <li key={artifactId}>
                    <EvidenceLink availability="unavailable" label={artifactId} reason={t("evaluation.artifactPreviewUnavailable")} to="" />
                  </li>
                ))}
              </ul>}
          </div>
          <div><h3 className="text-xs font-semibold">{t("evaluation.metrics")}</h3>{selected.metrics.map((item) => <p className="mt-1 text-xs" key={item.name}>{item.name}: {item.value ?? "—"} {item.unit} · {item.quality} · {item.source}</p>)}</div>
          <div><h3 className="text-xs font-semibold">{t("evaluation.timeline")}</h3>{selected.timeline.map((item) => <p className="mt-1 border-l-2 border-primary/50 pl-2 text-xs" key={item.id}>{item.label} · {item.status}</p>)}</div></div> : <p className="text-sm text-muted-foreground">{t("evaluation.selectResult")}</p>}
      </aside>
    </div>
    <EvaluationComparisonPanel attempts={allAttempts} />
  </div>;
}

// 18.12 "bounded reason": `check.summary` is free-text from the harness with no length constraint
// in `EvaluationCheck`, and the fixture generator (`evaluation-fixtures.ts`) already produces up to
// 20 checks per attempt. A "+N more" cap (Work Board's own `MAX_VISIBLE_SOURCES` precedent) was
// considered and rejected here specifically: checks are the primary pass/fail signal, and hiding
// some behind "+N more" could hide a *failing* check from view -- actively unsafe for a QA surface.
// Scroll-bound instead, mirroring the artifact list two sections below in this same detail pane:
// every check stays reachable, only the rendered height is capped. Each summary is additionally
// `line-clamp`-truncated with the full text kept in `title` (hover/long-press), never silently lost.
function Evidence({ attempt, title }: { attempt: EvaluationAttempt; title: string }) {
  return (
    <div>
      <h3 className="flex items-center gap-2 text-xs font-semibold"><ShieldCheck aria-hidden="true" className="h-4 w-4" />{title}</h3>
      <ul className="mt-1 flex max-h-48 flex-col gap-1 overflow-auto">
        {attempt.checks.map((check) => (
          <li className="text-xs" key={check.checkId}>
            <span className="font-medium">{check.passed ? "PASS" : "FAIL"}</span>
            {" · "}
            <span className="line-clamp-2 align-bottom" title={check.summary}>{check.summary}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
