import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatAppWeekdayNames } from "../i18n/format";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskFrequency } from "../types/agent";
import { ScheduledTaskDetail } from "./scheduled-task-detail";
import { ScheduledTaskForm } from "./scheduled-task-form";
import { ScheduledTaskList } from "./scheduled-task-list";
import { initialFrequency } from "./scheduled-task-presentation";

export interface ScheduledTasksPanelProps {
  agents: AgentRegistryEntry[];
  /** 19.3: the route's own current selection (`RunsSection`'s `scheduleId`, `workbench-route.ts`)
   *  -- the first real consumer of that field. It was parsed by `parseRunsSection` from day one
   *  but never read by anything (confirmed by `runs-destination.tsx`'s own prior audit note). */
  scheduleId?: string;
  /** Pushes a selection change back through `RunsDestination`'s `onSectionChange` into the URL via
   *  `runsPath()`, so Back/forward and reload restore the same selected task. Optional so this
   *  component still works standalone (e.g. in tests) without a routed parent. */
  onSelectSchedule?: (scheduleId: string | undefined) => void;
}

/**
 * 19.3: the composing container left behind after splitting list/detail/create-form into separate
 * components and a shared `scheduled-task-presentation.ts` primitive -- this file used to hold all
 * three inline (265 lines). It still owns every piece of state (task collection, create-form
 * draft, and now selection); only the rendering moved out. State reset and the initial task load
 * used to depend on the dialog's `open` prop; here they run once on mount instead, since a routed
 * page is only ever rendered while "open."
 */
export function ScheduledTasksPanel({ agents, onSelectSchedule, scheduleId }: ScheduledTasksPanelProps) {
  const { i18n } = useTranslation();
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");
  const [agentId, setAgentId] = useState("");
  const [frequency, setFrequency] = useState<ScheduledTaskFrequency>(initialFrequency("daily"));
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(scheduleId ?? null);
  const weekdayNames = useMemo(() => formatAppWeekdayNames(i18n.language), [i18n.language]);

  const selectableAgents = useMemo(
    () => agents.filter((agent) => agent.id === "onepiece" || agent.supportedInteractionModes.includes("cli")),
    [agents],
  );

  async function loadTasks() {
    setLoading(true);
    setError(null);
    try {
      setTasks(await agentService.listScheduledTasks());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    setAgentId(selectableAgents[0]?.id ?? "");
    void loadTasks();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- runs once on mount, matching the dialog's former "on open" reset
  }, []);

  // 19.3: restores the task selected the last time this section was left -- the same "route drives
  // selection" shape as MissionControl's own `initialRunId` effect. Re-runs if the route's own
  // scheduleId changes while this stays mounted (Loops/Schedules stay mounted across a Runs tab
  // switch per 5.13), not only at first mount.
  useEffect(() => {
    if (scheduleId) setSelectedId(scheduleId);
  }, [scheduleId]);

  function selectTask(taskId: string | undefined) {
    setSelectedId(taskId ?? null);
    onSelectSchedule?.(taskId);
  }

  async function createTask() {
    if (!name.trim() || !content.trim() || !agentId) return;
    setSaving(true);
    setError(null);
    try {
      await agentService.createScheduledTask({ name, content, agentId, frequency });
      setName("");
      setContent("");
      setFrequency(initialFrequency("daily"));
      await loadTasks();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  }

  async function setEnabled(task: ScheduledTask, enabled: boolean) {
    setError(null);
    try {
      const updated = await agentService.setScheduledTaskEnabled({ taskId: task.id, enabled });
      setTasks((current) => current.map((candidate) => (candidate.id === task.id ? updated : candidate)));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function deleteTask(task: ScheduledTask) {
    setConfirmingDeleteId(null);
    setError(null);
    try {
      await agentService.deleteScheduledTask(task.id);
      setTasks((current) => current.filter((candidate) => candidate.id !== task.id));
      if (task.id === selectedId) selectTask(undefined);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  const selected = tasks.find((task) => task.id === selectedId) ?? null;
  const selectedAgent = selected ? agents.find((agent) => agent.id === selected.agentId) : undefined;

  return (
    <div className="grid min-h-0 gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_320px]">
      <ScheduledTaskList
        agents={agents}
        confirmingDeleteId={confirmingDeleteId}
        language={i18n.language}
        loading={loading}
        onConfirmDelete={(task) => void deleteTask(task)}
        onRequestDelete={setConfirmingDeleteId}
        onSelect={selectTask}
        onSetEnabled={(task, enabled) => void setEnabled(task, enabled)}
        selectedId={selectedId}
        tasks={tasks}
        weekdayNames={weekdayNames}
      />
      <div className="grid content-start gap-4">
        <ScheduledTaskDetail agent={selectedAgent} language={i18n.language} task={selected} weekdayNames={weekdayNames} />
        <ScheduledTaskForm
          agentId={agentId}
          agents={selectableAgents}
          content={content}
          error={error}
          frequency={frequency}
          name={name}
          onAgentIdChange={setAgentId}
          onContentChange={setContent}
          onFrequencyChange={setFrequency}
          onNameChange={setName}
          onSubmit={() => void createTask()}
          saving={saving}
          weekdayNames={weekdayNames}
        />
      </div>
    </div>
  );
}
