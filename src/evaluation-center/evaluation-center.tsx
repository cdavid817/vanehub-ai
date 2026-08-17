import { useEffect, useMemo, useState } from "react";
import { Download, Play, ShieldCheck, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena, EvaluationAttempt, EvaluationTask } from "../types/evaluation";

const AGENTS = ["onepiece", "codex-cli"];
const TERMINAL = new Set(["succeeded", "task_failed", "agent_failed", "timed_out", "stuck", "cancelled", "benchmark_error"]);

export function EvaluationCenter() {
  const { t } = useTranslation();
  const [tasks, setTasks] = useState<EvaluationTask[]>([]);
  const [arenas, setArenas] = useState<EvaluationArena[]>([]);
  const [taskId, setTaskId] = useState("");
  const [agentIds, setAgentIds] = useState(AGENTS);
  const [selected, setSelected] = useState<EvaluationAttempt | null>(null);
  const [filter, setFilter] = useState("");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    async function loadInitial() {
      try {
        const [catalog, history] = await Promise.all([agentService.listEvaluationTasks(), agentService.listEvaluationArenas()]);
        setTasks(catalog); setTaskId(catalog[0]?.id ?? ""); setArenas(history);
      } catch { setError(t("evaluation.loadError")); }
    }
    void loadInitial();
  }, [t]);
  useEffect(() => {
    if (!arenas.some((arena) => arena.attempts.some((attempt) => !TERMINAL.has(attempt.outcome)))) return;
    const timer = window.setInterval(() => { void agentService.listEvaluationArenas().then(setArenas); }, 1_000);
    return () => window.clearInterval(timer);
  }, [arenas]);
  const activeTask = useMemo(() => tasks.find((task) => task.id === taskId), [taskId, tasks]);
  const visible = useMemo(() => arenas.flatMap((arena) => arena.attempts.map((attempt) => ({ arena, attempt })))
    .filter(({ attempt }) => `${attempt.agent.agentId} ${attempt.outcome}`.toLowerCase().includes(filter.toLowerCase())), [arenas, filter]);
  async function start() {
    if (!activeTask || agentIds.length === 0) return;
    setRunning(true); setError(null);
    try {
      const arena = await agentService.startEvaluation({ taskId: activeTask.id, taskVersion: activeTask.version, agentIds });
      setArenas((items) => [arena, ...items]); setSelected(arena.attempts[0] ?? null);
    } catch { setError(t("evaluation.runError")); } finally { setRunning(false); }
  }
  async function cancel() {
    if (!selected) return;
    try { const arena = await agentService.cancelEvaluation(selected.arenaId); replaceArena(arena); setSelected(arena.attempts.find((item) => item.id === selected.id) ?? null); }
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
      <select aria-label={t("evaluation.task")} className="h-9 rounded-md border border-input bg-background px-2 text-sm" onChange={(event) => setTaskId(event.target.value)} value={taskId}>{tasks.map((task) => <option key={task.id} value={task.id}>{task.id} v{task.version}</option>)}</select>
      <fieldset className="flex h-9 items-center gap-2 rounded-md border border-input px-2"><legend className="sr-only">{t("evaluation.agents")}</legend>{AGENTS.map((agent) => <label className="flex items-center gap-1 text-xs" key={agent}><input checked={agentIds.includes(agent)} onChange={() => setAgentIds((items) => items.includes(agent) ? items.filter((item) => item !== agent) : [...items, agent])} type="checkbox" />{agent}</label>)}</fieldset>
      <button className="ucd-button-primary flex h-9 items-center gap-2 rounded-md px-3 text-sm" disabled={!activeTask || running || agentIds.length === 0} onClick={() => void start()} type="button"><Play aria-hidden="true" className="h-4 w-4" />{running ? t("evaluation.running") : t("evaluation.run")}</button>
    </header>
    {error ? <p className="border-b border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive" role="alert">{error}</p> : null}
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-auto lg:grid-cols-[minmax(420px,1.3fr)_minmax(280px,0.7fr)]">
      <section className="min-w-0 border-r border-border p-3"><div className="mb-2 flex items-center justify-between gap-2"><h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.results")}</h2><input aria-label={t("evaluation.filter")} className="h-8 w-44 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => setFilter(event.target.value)} placeholder={t("evaluation.filter")} value={filter} /></div>
        <div className="overflow-x-auto"><table className="w-full text-left text-xs"><thead><tr className="border-b border-border text-muted-foreground"><th className="p-2">{t("evaluation.agent")}</th><th>{t("evaluation.outcome")}</th><th>{t("evaluation.tests")}</th><th>{t("evaluation.tokens")}</th><th>{t("evaluation.time")}</th><th><span className="sr-only">{t("evaluation.export")}</span></th></tr></thead><tbody>{visible.map(({ arena, attempt }) => <tr className="cursor-pointer border-b border-border/60 hover:bg-muted/60" key={attempt.id} onClick={() => setSelected(attempt)}><td className="p-2 font-medium">{attempt.agent.agentId}</td><td>{t(`evaluation.outcome.${attempt.outcome}`)}</td><td>{attempt.checks.filter((item) => item.passed).length}/{attempt.checks.length}</td><td>{metric(attempt, "input_tokens")}</td><td>{metric(attempt, "duration")}</td><td><button aria-label={t("evaluation.export")} className="rounded p-1 hover:bg-muted" onClick={(event) => { event.stopPropagation(); void exportArena(arena); }} type="button"><Download aria-hidden="true" className="h-4 w-4" /></button></td></tr>)}</tbody></table></div>
        {visible.length === 0 ? <p className="p-6 text-center text-sm text-muted-foreground">{t("evaluation.empty")}</p> : null}
      </section>
      <aside className="min-w-0 p-3"><div className="mb-2 flex items-center justify-between"><h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.detail")}</h2>{selected && !TERMINAL.has(selected.outcome) ? <button className="flex items-center gap-1 rounded-md border border-input px-2 py-1 text-xs" onClick={() => void cancel()} type="button"><Square className="h-3 w-3" />{t("evaluation.cancel")}</button> : null}</div>
        {selected ? <div className="space-y-3"><div className="rounded-md border border-border bg-muted/30 p-3"><p className="font-mono text-xs">{selected.agent.providerId} / {selected.agent.modelId ?? t("evaluation.unavailable")}</p><p className="mt-1 text-xs text-muted-foreground">{selected.agent.configurationFingerprint}</p></div>
          <Evidence attempt={selected} title={t("evaluation.verification")} /><div><h3 className="text-xs font-semibold">{t("evaluation.diff")}</h3><pre className="mt-1 max-h-32 overflow-auto rounded bg-muted p-2 text-[11px]">{selected.artifactIds.slice(0, 20).join("\n") || t("evaluation.unavailable")}</pre></div>
          <div><h3 className="text-xs font-semibold">{t("evaluation.metrics")}</h3>{selected.metrics.map((item) => <p className="mt-1 text-xs" key={item.name}>{item.name}: {item.value ?? "—"} {item.unit} · {item.quality} · {item.source}</p>)}</div>
          <div><h3 className="text-xs font-semibold">{t("evaluation.timeline")}</h3>{selected.timeline.map((item) => <p className="mt-1 border-l-2 border-primary/50 pl-2 text-xs" key={item.id}>{item.label} · {item.status}</p>)}</div></div> : <p className="text-sm text-muted-foreground">{t("evaluation.selectResult")}</p>}
      </aside>
    </div>
  </div>;
}

function Evidence({ attempt, title }: { attempt: EvaluationAttempt; title: string }) { return <div><h3 className="flex items-center gap-2 text-xs font-semibold"><ShieldCheck aria-hidden="true" className="h-4 w-4" />{title}</h3>{attempt.checks.map((check) => <p className="mt-1 text-xs" key={check.checkId}>{check.passed ? "PASS" : "FAIL"} · {check.summary}</p>)}</div>; }
function metric(attempt: EvaluationAttempt, name: string) { const value = attempt.metrics.find((item) => item.name === name); return value?.value == null ? "—" : `${value.value} ${value.unit}`; }
