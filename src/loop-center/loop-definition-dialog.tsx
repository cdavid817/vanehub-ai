import { useEffect, useRef, useState, type ReactNode } from "react";
import { ArrowLeft, ArrowRight, Loader2, Play, Save, X } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { useLoopBranchesQuery, useLoopProjectChoicesQuery } from "../hooks/use-loop-queries";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry } from "../types/agent";
import type { LoopBranchChoice, LoopDefinition, LoopLimits, LoopProjectChoice } from "../types/loop";
import { createLoopDefinitionDraft, toSaveLoopDefinitionInput, validateLoopDefinitionStep, type LoopDefinitionDraft } from "./loop-definition-form";
import { LoopVerificationCommandEditor } from "./loop-verification-command-editor";

interface LoopDefinitionDialogProps {
  definition: LoopDefinition | null;
  onClose: () => void;
  onSaved: (definition: LoopDefinition, requestStart: boolean) => void;
}

const steps = ["scope", "agents", "verification", "review"] as const;

export function LoopDefinitionDialog({ definition, onClose, onSaved }: LoopDefinitionDialogProps) {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDivElement>(null);
  const agents = useQuery({ queryKey: ["agents", "loops"], queryFn: () => agentService.listAgents() });
  const projects = useLoopProjectChoicesQuery();
  const [draft, setDraft] = useState(() => createLoopDefinitionDraft(definition));
  const branches = useLoopBranchesQuery(draft.projectPath);
  const [step, setStep] = useState(0);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showStepErrors, setShowStepErrors] = useState(false);
  const [persistedDefinition, setPersistedDefinition] = useState<LoopDefinition | null>(null);
  // Editing invalidates whatever the footer complained about, so the message must not outlive the
  // input it described; the inline per-command errors keep re-validating live on their own.
  const updateDraft = (next: LoopDefinitionDraft) => { setDraft(next); setPersistedDefinition(null); setError(null); };

  useEffect(() => {
    const registered = agents.data ?? [];
    if (registered.length === 0) return;
    // The backend refuses to save a definition whose agent is not selectable, so preselecting the
    // registry's first entry regardless of availability steered every fresh wizard into a rejection
    // on hosts without that CLI. Prefer agents that can actually run.
    const usable = registered.filter((agent) => agent.availabilityState === "available");
    const pool = usable.length > 0 ? usable : registered;
    setDraft((current) => ({
      ...current,
      workerAgentId: current.workerAgentId || pool[0].id,
      verifierAgentId: current.verifierAgentId || pool[1]?.id || pool[0].id,
    }));
  }, [agents.data]);

  useEffect(() => {
    const first = projects.data?.find((project) => project.available);
    if (!first) return;
    setDraft((current) => current.projectPath ? current : { ...current, projectPath: first.path });
  }, [projects.data]);

  useEffect(() => {
    const panel = panelRef.current;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    panel?.querySelector<HTMLElement>("input, textarea, select, button")?.focus();
    return () => previous?.focus();
  }, []);

  useEffect(() => {
    const panel = panelRef.current;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !saving) onClose();
      if (event.key !== "Tab" || !panel) return;
      const items = Array.from(panel.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'));
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose, saving]);

  function next() {
    const issue = validateLoopDefinitionStep(draft, step);
    if (issue) { setShowStepErrors(true); setError(t(`loops.editor.error.${issue}`)); return; }
    setError(null);
    setShowStepErrors(false);
    setStep((current) => Math.min(current + 1, steps.length - 1));
  }

  async function submit(start: boolean) {
    for (let current = 0; current < 3; current += 1) {
      const issue = validateLoopDefinitionStep(draft, current);
      if (issue) { setStep(current); setShowStepErrors(true); setError(t(`loops.editor.error.${issue}`)); return; }
    }
    setSaving(true);
    setError(null);
    try {
      let saved = persistedDefinition;
      if (!saved) {
        const input = toSaveLoopDefinitionInput(draft, definition);
        saved = definition
          ? await agentService.updateLoopDefinition(definition.id, input)
          : await agentService.createLoopDefinition(input);
        setPersistedDefinition(saved);
      }
      onSaved(saved, start);
    } catch (submitError) {
      const message = submitError instanceof Error ? submitError.message : String(submitError);
      // The backend refuses agents that cannot run (loop_service.rs validate_agent) with an
      // untranslated message and no hint about where to act. Send the user to the step that can
      // fix it and keep the backend detail as the explanation.
      if (message.startsWith("agent is unavailable")) {
        setStep(1);
        setError(t("loops.editor.error.agentUnavailable", { detail: `${message.slice("agent is unavailable:".length).trim()} ` }));
      } else {
        setError(message);
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/75 p-3 sm:p-4">
      <div aria-labelledby="loop-editor-title" aria-modal="true" className="ucd-panel grid max-h-[92vh] w-full max-w-3xl grid-rows-[auto_auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg shadow-xl" ref={panelRef} role="dialog">
        <header className="flex h-14 items-center gap-3 border-b border-border px-4">
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold" id="loop-editor-title">{t(definition ? "loops.editor.editTitle" : "loops.editor.createTitle")}</h2>
            <p className="truncate text-xs text-muted-foreground">{t(`loops.editor.step.${steps[step]}`)}</p>
          </div>
          <Button aria-label={t("loops.editor.close")} disabled={saving} onClick={onClose} size="icon" title={t("loops.editor.close")} type="button" variant="ghost"><X aria-hidden="true" /></Button>
        </header>
        <ol aria-label={t("loops.editor.progress")} className="grid grid-cols-4 border-b border-border bg-muted/25 px-2 sm:px-4">
          {steps.map((value, index) => <li className={`border-b-2 px-1 py-2 text-center text-[11px] font-medium sm:text-xs ${index === step ? "border-primary text-foreground" : "border-transparent text-muted-foreground"}`} key={value}>{index + 1}. {t(`loops.editor.step.${value}`)}</li>)}
        </ol>
        <div className="min-h-0 overflow-y-auto p-4 sm:p-5">
          {step === 0 ? <ScopeStep branches={branches.data ?? []} draft={draft} loading={projects.isLoading || branches.isFetching} projects={projects.data ?? []} setDraft={updateDraft} /> : null}
          {step === 1 ? <AgentsStep agents={agents.data ?? []} draft={draft} loading={agents.isLoading} setDraft={updateDraft} /> : null}
          {step === 2 ? <VerificationStep draft={draft} setDraft={updateDraft} showErrors={showStepErrors} /> : null}
          {step === 3 ? <ReviewStep agents={agents.data ?? []} draft={draft} /> : null}
        </div>
        <footer className="flex min-h-14 items-center justify-between gap-3 border-t border-border px-4 py-2">
          <p aria-live="polite" className="line-clamp-3 min-w-0 flex-1 wrap-break-word text-xs text-destructive" title={error ?? undefined}>{error}</p>
          <div className="flex shrink-0 gap-2">
            {step > 0 ? <Button disabled={saving} onClick={() => { setError(null); setShowStepErrors(false); setStep(step - 1); }} size="sm" type="button" variant="outline"><ArrowLeft aria-hidden="true" />{t("loops.editor.back")}</Button> : null}
            {step < 3 ? <Button onClick={next} size="sm" type="button">{t("loops.editor.next")}<ArrowRight aria-hidden="true" /></Button> : <>
              <Button disabled={saving} onClick={() => void submit(false)} size="sm" type="button" variant="outline">{saving ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Save aria-hidden="true" />}{t("loops.editor.save")}</Button>
              <Button disabled={saving} onClick={() => void submit(true)} size="sm" type="button">{saving ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Play aria-hidden="true" />}{t("loops.editor.saveAndRun")}</Button>
            </>}
          </div>
        </footer>
      </div>
    </div>
  );
}

// `<option>` labels can only be plain text, so a discovered-but-simulated (Web mock) choice is
// distinguished the same textual-suffix way as an unavailable one is, not with the badge
// `loop-run-header.tsx`/`loop-preflight-dialog.tsx` render for the same `loops.simulated` string
// once a real selection has been made — 17.6 needs the distinction visible at pick time too, not
// only after picking.
function optionSuffix(choice: { available: boolean; simulated: boolean }, t: (key: string) => string): string {
  const parts = [
    choice.available ? null : t("loops.editor.unavailable"),
    choice.simulated ? t("loops.simulated") : null,
  ].filter((part): part is string => part !== null);
  return parts.length ? ` — ${parts.join(", ")}` : "";
}

function ScopeStep({ branches, draft, loading, projects, setDraft }: StepProps & { branches: LoopBranchChoice[]; loading: boolean; projects: LoopProjectChoice[] }) {
  const { t } = useTranslation();
  const projectOptions = retainProjectChoice(projects, draft.projectPath);
  const branchOptions = retainBranchChoice(branches, draft.baseBranch);
  return <div className="grid gap-4 sm:grid-cols-2">
    <Field label="loops.editor.field.name"><input className={inputClass} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
    <Field label="loops.editor.field.project"><select className={inputClass} disabled={loading && projectOptions.length === 0} value={draft.projectPath} onChange={(event) => setDraft({ ...draft, projectPath: event.target.value })}><option value="">{t("loops.editor.selectProject")}</option>{projectOptions.map((project) => <option key={project.path} value={project.path}>{project.displayName}{optionSuffix(project, t)}</option>)}</select></Field>
    <Field label="loops.editor.field.branch"><select className={inputClass} disabled={!draft.projectPath || (loading && branchOptions.length === 0)} value={draft.baseBranch} onChange={(event) => setDraft({ ...draft, baseBranch: event.target.value })}><option value="">{t("loops.editor.selectBranch")}</option>{branchOptions.map((branch) => <option key={`${branch.kind}:${branch.name}`} value={branch.name}>{branch.name}{optionSuffix(branch, t)}</option>)}</select></Field>
    <label className="flex min-h-9 items-center gap-2 text-sm"><input checked={draft.enabled} className="h-4 w-4 accent-primary" onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} type="checkbox" /><span>{t("loops.editor.field.enabled")}</span></label>
    <Field className="sm:col-span-2" label="loops.editor.field.goal"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.goal} onChange={(event) => setDraft({ ...draft, goal: event.target.value })} /></Field>
    <Field className="sm:col-span-2" label="loops.editor.field.acceptance"><textarea className={`${inputClass} min-h-24 py-2`} value={draft.acceptanceCriteria} onChange={(event) => setDraft({ ...draft, acceptanceCriteria: event.target.value })} /></Field>
    <Field label="loops.editor.field.allowedPaths"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.allowedPaths} onChange={(event) => setDraft({ ...draft, allowedPaths: event.target.value })} /></Field>
    <Field label="loops.editor.field.protectedPaths"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.protectedPaths} onChange={(event) => setDraft({ ...draft, protectedPaths: event.target.value })} /></Field>
  </div>;
}

function AgentsStep({ agents, draft, loading, setDraft }: StepProps & { agents: AgentRegistryEntry[]; loading: boolean }) {
  const { t } = useTranslation();
  // Same convention as the project and branch selects: an entry the backend would refuse stays
  // pickable (its CLI may be installed later) but says so instead of failing at save time.
  const optionLabel = (agent: AgentRegistryEntry) => agent.availabilityState === "available"
    ? agent.displayName
    : `${agent.displayName} — ${t(`createSession.agentAvailability.${agent.availabilityState}`)}`;
  return <div className="grid gap-4 sm:grid-cols-2">
    <Field label="loops.editor.field.worker"><select className={inputClass} disabled={loading} value={draft.workerAgentId} onChange={(event) => setDraft({ ...draft, workerAgentId: event.target.value })}><option value="">{t("loops.editor.selectAgent")}</option>{agents.map((agent) => <option key={agent.id} value={agent.id}>{optionLabel(agent)}</option>)}</select></Field>
    <Field label="loops.editor.field.verifier"><select className={inputClass} disabled={loading} value={draft.verifierAgentId} onChange={(event) => setDraft({ ...draft, verifierAgentId: event.target.value })}><option value="">{t("loops.editor.selectAgent")}</option>{agents.map((agent) => <option key={agent.id} value={agent.id}>{optionLabel(agent)}</option>)}</select></Field>
  </div>;
}

function VerificationStep({ draft, setDraft, showErrors }: StepProps & { showErrors: boolean }) {
  return <div className="grid gap-5">
    <LoopVerificationCommandEditor commands={draft.verificationCommands} onChange={(verificationCommands) => setDraft({ ...draft, verificationCommands })} showErrors={showErrors} />
    <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <NumberField field="maxIterations" max={20} draft={draft} setDraft={setDraft} />
      <NumberField field="stepTimeoutSeconds" draft={draft} setDraft={setDraft} />
      <NumberField field="totalTimeoutSeconds" draft={draft} setDraft={setDraft} />
      <NumberField field="maxConsecutiveRuntimeErrors" draft={draft} setDraft={setDraft} />
      <NumberField field="maxConsecutiveNoProgress" draft={draft} setDraft={setDraft} />
    </section>
  </div>;
}

function ReviewStep({ agents, draft }: { agents: AgentRegistryEntry[]; draft: LoopDefinitionDraft }) {
  const { t } = useTranslation();
  const name = (id: string) => agents.find((agent) => agent.id === id)?.displayName ?? id;
  const commands = draft.verificationCommands.map((command) => `${command.program} ${command.arguments.split(/\r?\n/).filter(Boolean).join(" ")}`.trim()).join("; ");
  const rows = [
    ["name", draft.name], ["enabled", t(draft.enabled ? "loops.definition.enabled" : "loops.definition.disabled")],
    ["project", draft.projectPath], ["branch", draft.baseBranch], ["goal", draft.goal], ["acceptance", draft.acceptanceCriteria],
    ["allowedPaths", draft.allowedPaths], ["protectedPaths", draft.protectedPaths], ["worker", name(draft.workerAgentId)],
    ["verifier", name(draft.verifierAgentId)], ["commands", commands], ["maxIterations", String(draft.limits.maxIterations)],
    ["stepTimeoutSeconds", String(draft.limits.stepTimeoutSeconds)], ["totalTimeoutSeconds", String(draft.limits.totalTimeoutSeconds)],
    ["maxConsecutiveRuntimeErrors", String(draft.limits.maxConsecutiveRuntimeErrors)], ["maxConsecutiveNoProgress", String(draft.limits.maxConsecutiveNoProgress)],
  ];
  return <div className="grid gap-4"><dl className="grid gap-x-6 gap-y-3 sm:grid-cols-[minmax(8rem,auto)_1fr]">{rows.map(([key, value]) => <div className="contents" key={key}><dt className="text-xs font-medium text-muted-foreground">{t(`loops.editor.field.${key}`)}</dt><dd className="wrap-break-word whitespace-pre-line text-sm">{value}</dd></div>)}</dl><div className="border-t border-border pt-3 text-xs leading-5 text-muted-foreground"><p>{t("loops.editor.review.worktree")}</p><p className="mt-1 font-medium text-foreground">{t("loops.editor.review.humanGate")}</p></div></div>;
}

interface StepProps { draft: LoopDefinitionDraft; setDraft: (draft: LoopDefinitionDraft) => void }
const inputClass = "ucd-input h-9 w-full rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function Field({ children, className = "", label }: { children: ReactNode; className?: string; label: string }) {
  const { t } = useTranslation();
  return <label className={`grid gap-1.5 ${className}`}><span className="text-xs font-medium text-muted-foreground">{t(label)}</span>{children}</label>;
}

function NumberField({ draft, field, max, setDraft }: StepProps & { field: keyof LoopLimits; max?: number }) {
  return <Field label={`loops.editor.field.${field}`}><input className={inputClass} max={max} min={1} type="number" value={draft.limits[field]} onChange={(event) => setDraft({ ...draft, limits: { ...draft.limits, [field]: Number(event.target.value) } })} /></Field>;
}

function retainProjectChoice(choices: LoopProjectChoice[], value: string) {
  if (!value || choices.some((choice) => choice.path === value)) return choices;
  return [{ path: value, displayName: value, available: false, simulated: false }, ...choices];
}

function retainBranchChoice(choices: LoopBranchChoice[], value: string) {
  if (!value || choices.some((choice) => choice.name === value)) return choices;
  return [{ name: value, kind: "local" as const, available: false, simulated: false }, ...choices];
}
