import { useEffect, useMemo, useState } from "react";
import { ShieldCheck, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena, EvaluationAttempt } from "../types/evaluation";
import { EvidenceLink } from "../ui/evidence/EvidenceLink";
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
  const activeTask = useMemo(() => tasks.find((task) => task.id === taskId), [taskId, tasks]);
  // Derived rather than held: an attempt captured at click time is a snapshot of a run still in
  // flight, and the polling below replaces the arena it came from without ever touching it. The
  // detail pane went on showing `queued` -- Cancel button and all -- beside a row that had already
  // reported its verdict.
  const selected = useMemo(
    () => arenas.flatMap((arena) => arena.attempts).find((attempt) => attempt.id === selectedId) ?? null,
    [arenas, selectedId],
  );
  const visible = useMemo(() => arenas.flatMap((arena) => arena.attempts.map((attempt) => ({ arena, attempt })))
    .filter(({ attempt }) => `${attempt.agent.agentId} ${attempt.outcome}`.toLowerCase().includes(filter.toLowerCase())), [arenas, filter]);
  function toggleAgent(agentId: string) {
    setAgentIds((items) => items.includes(agentId) ? items.filter((item) => item !== agentId) : [...items, agentId]);
  }
  async function start() {
    if (!activeTask || agentIds.length === 0) return;
    setRunning(true); setError(null);
    try {
      const arena = await agentService.startEvaluation({ taskId: activeTask.id, taskVersion: activeTask.version, agentIds });
      setArenas((items) => [arena, ...items]); setSelectedId(arena.attempts[0]?.id ?? null);
    } catch { setError(t("evaluation.runError")); } finally { setRunning(false); }
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
        disabled={!activeTask || running || agentIds.length === 0}
        onRun={() => void start()}
        onTaskIdChange={setTaskId}
        onToggleAgent={toggleAgent}
        running={running}
        taskId={taskId}
        tasks={tasks}
      />
    </header>
    {error ? <p className="border-b border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive" role="alert">{error}</p> : null}
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
  </div>;
}

function Evidence({ attempt, title }: { attempt: EvaluationAttempt; title: string }) { return <div><h3 className="flex items-center gap-2 text-xs font-semibold"><ShieldCheck aria-hidden="true" className="h-4 w-4" />{title}</h3>{attempt.checks.map((check) => <p className="mt-1 text-xs" key={check.checkId}>{check.passed ? "PASS" : "FAIL"} · {check.summary}</p>)}</div>; }
