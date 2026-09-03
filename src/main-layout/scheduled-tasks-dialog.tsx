import { Loader2, Plus } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { ScheduledTaskForm } from "./scheduled-task-form";
import { ScheduledTaskList, type ScheduledTaskMutation } from "./scheduled-task-list";
import { initialScheduledTaskDraft, isValidScheduledTaskDraft } from "./scheduled-task-model";

function sortTasks(tasks: ScheduledTask[]) {
  return [...tasks].sort((left, right) => left.nextRunAt.localeCompare(right.nextRunAt));
}

export function ScheduledTasksDialog({
  agents,
  onClose,
  open,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  open: boolean;
}) {
  const { t } = useTranslation();
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

  useEffect(() => {
    if (!open) return undefined;
    let active = true;
    setDraft(initialScheduledTaskDraft(defaultAgentIdRef.current));
    setConfirmingDeleteId(null);
    setLoading(true);
    setError(null);
    void agentService.listScheduledTasks()
      .then((loaded) => {
        if (active) setTasks(sortTasks(loaded));
      })
      .catch((reason: unknown) => {
        if (active) setError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => { active = false; };
  }, [open]);

  useEffect(() => {
    if (!open || draft.agentId || !defaultAgentIdRef.current) return;
    setDraft((current) => ({ ...current, agentId: defaultAgentIdRef.current }));
  }, [draft.agentId, open, selectableAgents]);

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

  if (!open) return null;
  const busy = saving || mutation !== null;

  return (
    <ApplicationDialog
      closeDisabled={busy}
      description={t("scheduledTasks.description")}
      footer={(
        <div className="flex min-h-8 items-center justify-between gap-3">
          <p className="min-w-0 flex-1 wrap-break-word text-xs leading-5 text-destructive" role={error ? "alert" : undefined}>{error}</p>
          <Button className="h-8 shrink-0 px-3 text-xs" disabled={!isValidScheduledTaskDraft(draft) || busy} onClick={() => void createTask()} type="button">
            {saving ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Plus aria-hidden="true" />}
            {saving ? t("scheduledTasks.creating") : t("scheduledTasks.create")}
          </Button>
        </div>
      )}
      maxWidth="max-w-6xl"
      onClose={onClose}
      title={t("scheduledTasks.title")}
    >
      <div className="grid min-h-0 gap-5 lg:grid-cols-[minmax(0,1fr)_360px]">
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
        <ScheduledTaskForm agents={selectableAgents} disabled={busy} draft={draft} onChange={setDraft} />
      </div>
    </ApplicationDialog>
  );
}
