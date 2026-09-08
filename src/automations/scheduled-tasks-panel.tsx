import { Loader2, Plus } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { ScheduledTaskForm } from "../main-layout/scheduled-task-form";
import { ScheduledTaskList, type ScheduledTaskMutation } from "../main-layout/scheduled-task-list";
import { initialScheduledTaskDraft, isValidScheduledTaskDraft } from "../main-layout/scheduled-task-model";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";

function sortTasks(tasks: ScheduledTask[]) {
  return [...tasks].sort((left, right) => left.nextRunAt.localeCompare(right.nextRunAt));
}

const firstControlSelector = 'input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled])';

/**
 * Scheduled-task management as page content. The form, validation, list, refresh, mutation, and
 * error-retention behavior are the dialog's, unchanged; only the container differs — no modal, no
 * open/close lifecycle, and focus lands on the surface's first control when the tab is shown.
 */
export function ScheduledTasksPanel({ active = true, agents }: { active?: boolean; agents: AgentRegistryEntry[] }) {
  const { t } = useTranslation();
  const rootRef = useRef<HTMLElement>(null);
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [draft, setDraft] = useState(initialScheduledTaskDraft);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mutation, setMutation] = useState<ScheduledTaskMutation | null>(null);
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<string | null>(null);

  const selectableAgents = useMemo(
    () => agents.filter((agent) => agent.id === "onepiece" || agent.supportedInteractionModes.includes("cli")),
    [agents],
  );
  const defaultAgentIdRef = useRef("");
  defaultAgentIdRef.current = selectableAgents[0]?.id ?? "";

  // Loaded once for the life of the surface: switching tabs hides it rather than unmounting it,
  // which is what keeps the list and an unsubmitted draft in place.
  useEffect(() => {
    let mounted = true;
    setDraft(initialScheduledTaskDraft(defaultAgentIdRef.current));
    setLoading(true);
    setError(null);
    void agentService.listScheduledTasks()
      .then((loaded) => {
        if (mounted) setTasks(sortTasks(loaded));
      })
      .catch((reason: unknown) => {
        if (mounted) setError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => { mounted = false; };
  }, []);

  useEffect(() => {
    if (draft.agentId || !defaultAgentIdRef.current) return;
    setDraft((current) => ({ ...current, agentId: defaultAgentIdRef.current }));
  }, [draft.agentId, selectableAgents]);

  useEffect(() => {
    if (!active) return;
    rootRef.current?.querySelector<HTMLElement>(firstControlSelector)?.focus();
  }, [active]);

  async function createTask() {
    if (!isValidScheduledTaskDraft(draft)) return;
    setSaving(true);
    setError(null);
    try {
      const created = await agentService.createScheduledTask({
        agentId: draft.agentId,
        content: draft.content.trim(),
        frequency: draft.frequency,
        name: draft.name.trim(),
      });
      setTasks((current) => sortTasks([created, ...current]));
      setDraft(initialScheduledTaskDraft(draft.agentId));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  }

  async function setEnabled(task: ScheduledTask, enabled: boolean) {
    setMutation({ action: enabled ? "enable" : "disable", taskId: task.id });
    setError(null);
    try {
      const updated = await agentService.setScheduledTaskEnabled({ taskId: task.id, enabled });
      setTasks((current) => current.map((candidate) => (candidate.id === task.id ? updated : candidate)));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setMutation(null);
    }
  }

  async function deleteTask(task: ScheduledTask) {
    setConfirmingDeleteId(null);
    setMutation({ action: "delete", taskId: task.id });
    setError(null);
    try {
      await agentService.deleteScheduledTask(task.id);
      setTasks((current) => current.filter((candidate) => candidate.id !== task.id));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setMutation(null);
    }
  }

  const busy = saving || mutation !== null;

  return (
    <section aria-labelledby="scheduled-tasks-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" data-testid="scheduled-tasks-panel" id="scheduled-tasks" ref={rootRef}>
      <header className="shrink-0 border-b border-border p-3 md:p-4">
        <h1 className="text-base font-semibold" id="scheduled-tasks-title">{t("scheduledTasks.title")}</h1>
        <p className="text-xs text-muted-foreground">{t("scheduledTasks.description")}</p>
      </header>
      {/* Form first in DOM order so the surface's first control is the task name; the list's own
          order classes still place it on the left at desktop width. */}
      <div className="grid min-h-0 flex-1 gap-5 overflow-y-auto p-3 md:p-4 lg:grid-cols-[minmax(0,1fr)_360px]">
        <ScheduledTaskForm agents={selectableAgents} disabled={busy} draft={draft} onChange={setDraft} />
        <ScheduledTaskList
          agents={agents}
          confirmingDeleteId={confirmingDeleteId}
          loading={loading}
          mutation={mutation}
          onCancelDelete={() => setConfirmingDeleteId(null)}
          onConfirmDelete={(task) => void deleteTask(task)}
          onRequestDelete={setConfirmingDeleteId}
          onSetEnabled={(task, enabled) => void setEnabled(task, enabled)}
          tasks={tasks}
        />
      </div>
      <footer className="flex min-h-8 shrink-0 items-center justify-between gap-3 border-t border-border p-3">
        <p className="min-w-0 flex-1 wrap-break-word text-xs leading-5 text-destructive" role={error ? "alert" : undefined}>{error}</p>
        <Button className="h-8 shrink-0 px-3 text-xs" disabled={!isValidScheduledTaskDraft(draft) || busy} onClick={() => void createTask()} type="button">
          {saving ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Plus aria-hidden="true" />}
          {saving ? t("scheduledTasks.creating") : t("scheduledTasks.create")}
        </Button>
      </footer>
    </section>
  );
}
