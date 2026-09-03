import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatAppWeekdayNames } from "../i18n/format";
import type { AgentRegistryEntry } from "../types/agent";
import { ScheduledTaskDetail } from "./scheduled-task-detail";
import { ScheduledTaskEditorSheet, type ScheduledTaskEditorMode } from "./scheduled-task-editor-sheet";
import { ScheduledTaskList } from "./scheduled-task-list";
import { SCHEDULED_TASK_CREATE_MUTATION_KEY, useScheduledTasksActions } from "./use-scheduled-tasks-actions";

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
 * 19.3's composing container, restructured by 19.7/19.9/19.16/19.17: mutation state (per-task
 * pending/error, including Create/Duplicate's own slot) now lives in `useScheduledTasksActions`
 * instead of this component's own ad hoc `saving`/`error`/`confirmingDeleteId`/`runningTaskId`
 * state, so selection, the open editor sheet, and every *other* row's own state all survive any
 * one row's mutation untouched -- see that hook's own doc comment for why one grouping bug
 * (Enable/Disable and Delete errors funneling into the create form's single `error` slot,
 * regardless of which row actually failed) motivated the change (19.17).
 */
export function ScheduledTasksPanel({ agents, onSelectSchedule, scheduleId }: ScheduledTasksPanelProps) {
  const { i18n } = useTranslation();
  const { create, error, load, loading, mutations, remove, runNow, setEnabled, tasks, update } = useScheduledTasksActions();
  const [selectedId, setSelectedId] = useState<string | null>(scheduleId ?? null);
  const [editorMode, setEditorMode] = useState<ScheduledTaskEditorMode | null>(null);
  const weekdayNames = useMemo(() => formatAppWeekdayNames(i18n.language), [i18n.language]);

  const selectableAgents = useMemo(
    () => agents.filter((agent) => agent.id === "onepiece" || agent.supportedInteractionModes.includes("cli")),
    [agents],
  );

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

  /** Opening any editor mode clears whatever this target's mutation slot last held, so a stale
   *  error from an earlier, unrelated action (e.g. a previous failed Delete) never leaks into a
   *  freshly opened sheet that has not attempted anything yet. */
  function openEditor(next: ScheduledTaskEditorMode) {
    mutations.clear(next.kind === "edit" ? next.task.id : SCHEDULED_TASK_CREATE_MUTATION_KEY);
    setEditorMode(next);
  }

  const selected = tasks.find((task) => task.id === selectedId) ?? null;
  const selectedAgent = selected ? agents.find((agent) => agent.id === selected.agentId) : undefined;
  const selectedMutation = selected ? mutations.get(selected.id) : undefined;
  const editorMutation = editorMode
    ? mutations.get(editorMode.kind === "edit" ? editorMode.task.id : SCHEDULED_TASK_CREATE_MUTATION_KEY)
    : undefined;

  return (
    <div className="grid min-h-0 gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_320px]">
      {error ? <p className="rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive lg:col-span-2" role="alert">{error}</p> : null}
      <ScheduledTaskList
        agents={agents}
        getMutation={mutations.get}
        language={i18n.language}
        loading={loading}
        onDelete={(task) => void remove(task, () => { if (task.id === selectedId) selectTask(undefined); })}
        onDismissError={mutations.clear}
        onDuplicate={(task) => openEditor({ kind: "duplicate", source: task })}
        onEdit={(task) => openEditor({ kind: "edit", task })}
        onNew={() => openEditor({ kind: "create" })}
        onSelect={selectTask}
        onSetEnabled={(task, enabled) => void setEnabled(task, enabled)}
        selectedId={selectedId}
        tasks={tasks}
        weekdayNames={weekdayNames}
      />
      <div className="grid content-start gap-4">
        <ScheduledTaskDetail
          agent={selectedAgent}
          isRunningNow={selected !== null && (selectedMutation?.pending ?? false)}
          language={i18n.language}
          onRunNow={() => selected && void runNow(selected)}
          runNowError={selectedMutation?.error?.message ?? null}
          task={selected}
          weekdayNames={weekdayNames}
        />
      </div>
      {editorMode ? (
        <ScheduledTaskEditorSheet
          agents={selectableAgents}
          mode={editorMode}
          mutation={editorMutation}
          onClose={() => setEditorMode(null)}
          onCreate={create}
          onCreated={(task) => selectTask(task.id)}
          onReload={load}
          onUpdate={update}
          weekdayNames={weekdayNames}
        />
      ) : null}
    </div>
  );
}
