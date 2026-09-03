import { useState } from "react";
import { Loader2, Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import { Sheet } from "../ui/sheet/Sheet";
import type { AgentRegistryEntry, CreateScheduledTaskInput, ScheduledTask, UpdateScheduledTaskInput } from "../types/agent";
import {
  blankScheduledTaskDraft, duplicateScheduledTaskDraft, scheduledTaskDraftFromTask, toCreateScheduledTaskInput,
  toUpdateScheduledTaskInput, validateScheduledTaskDraft, type ScheduledTaskDraft,
} from "./scheduled-task-draft";
import { ScheduledTaskForm } from "./scheduled-task-form";
import { ScheduledTaskReview } from "./scheduled-task-review";
import { isScheduledTaskVersionConflict } from "./use-scheduled-tasks-actions";

/** 19.7/19.9: one sheet, three ways to arrive at it. `duplicate` and `edit` both carry the
 *  `ScheduledTask` they read from, but only `edit`'s is a live mutation target (version-checked,
 *  updated in place) -- `duplicate`'s `source` is a one-time prefill template for a brand-new
 *  row and is never sent back to the server (see `scheduled-task-draft.ts`'s own doc comment). */
export type ScheduledTaskEditorMode =
  | { kind: "create" }
  | { kind: "duplicate"; source: ScheduledTask }
  | { kind: "edit"; task: ScheduledTask };

export interface ScheduledTaskEditorSheetProps {
  agents: AgentRegistryEntry[];
  mode: ScheduledTaskEditorMode;
  /** This sheet's own in-flight create/update, if any -- `SCHEDULED_TASK_CREATE_MUTATION_KEY` for
   *  `create`/`duplicate`, `mutations.get(task.id)` for `edit` (see
   *  `use-scheduled-tasks-actions.ts`). Drives Save's disabled state and the pending spinner. */
  mutation: MutationState | undefined;
  weekdayNames: string[];
  onClose: () => void;
  onCreate: (input: CreateScheduledTaskInput) => Promise<ScheduledTask>;
  /** Selects the newly created/duplicated row, mirroring `goal-center.tsx`'s own
   *  `create(input, (goal) => selectGoal(goal.id))`. */
  onCreated?: (task: ScheduledTask) => void;
  /** The hook's own `load` -- re-fetches the canonical list on a version conflict so every other
   *  view (the row list behind this sheet) also moves forward, matching
   *  `loop-definition-dialog.tsx`'s own `handleVersionConflict` writing the refreshed list back
   *  into the shared query cache. */
  onReload: () => Promise<ScheduledTask[] | null>;
  onUpdate: (task: ScheduledTask, input: UpdateScheduledTaskInput) => Promise<ScheduledTask>;
}

function initialDraft(mode: ScheduledTaskEditorMode, defaultAgentId: string, copyName: (name: string) => string): ScheduledTaskDraft {
  if (mode.kind === "edit") return scheduledTaskDraftFromTask(mode.task);
  if (mode.kind === "duplicate") return duplicateScheduledTaskDraft(mode.source, copyName(mode.source.name));
  return blankScheduledTaskDraft(defaultAgentId);
}

/**
 * 19.7: Create and Edit share this one Sheet-mounted editor (Goal Center's `GoalForm`-in-`Sheet`
 * precedent, goal-center.tsx) instead of the former create-only, always-inline form. 19.9's
 * Duplicate is not a separate flow -- it opens this exact component in `create` mode with a
 * prefilled draft, so the reader sees and can still edit the full form and Review before anything
 * is created (unlike Loop Center's own Duplicate, which creates immediately after a bare rename
 * confirm -- see `loop-definition-overview.tsx`'s `useDuplicateLoopDefinitionMutation`).
 *
 * Version-conflict recovery (Edit only) mirrors `loop-definition-dialog.tsx`'s own `baseline`
 * split exactly: `draft` (what the reader typed) is seeded once from `mode` and never
 * overwritten -- "do not silently overwrite" means the reader's own edits survive a conflict.
 * `baseline` (which server row a retry is checked against) starts as `mode.task` and only ever
 * moves forward, via this component's own refetch on conflict, kept local rather than derived
 * from a parent list so a concurrent delete elsewhere can still be *explained* here instead of
 * silently unmounting this sheet the instant the row leaves the parent's own list.
 */
export function ScheduledTaskEditorSheet({ agents, mode, mutation, onClose, onCreate, onCreated, onReload, onUpdate, weekdayNames }: ScheduledTaskEditorSheetProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<ScheduledTaskDraft>(
    () => initialDraft(mode, agents[0]?.id ?? "", (name) => t("scheduledTasks.copyName", { name })),
  );
  const [baseline, setBaseline] = useState<ScheduledTask | null>(mode.kind === "edit" ? mode.task : null);
  const [conflictMessage, setConflictMessage] = useState<string | null>(null);
  const pending = mutation?.pending ?? false;
  const issue = validateScheduledTaskDraft(draft, agents);
  const removed = mode.kind === "edit" && baseline === null;

  const title = mode.kind === "edit"
    ? t("scheduledTasks.editor.editTitle", { name: mode.task.name })
    : t("scheduledTasks.createTitle");

  async function submit() {
    if (issue || pending) return;
    setConflictMessage(null);
    try {
      if (mode.kind === "edit") {
        if (!baseline) return;
        await onUpdate(baseline, toUpdateScheduledTaskInput(baseline, draft));
      } else {
        onCreated?.(await onCreate(toCreateScheduledTaskInput(draft)));
      }
      onClose();
    } catch (reason) {
      if (mode.kind !== "edit" || !isScheduledTaskVersionConflict(reason)) return;
      const fresh = await onReload();
      const match = fresh?.find((task) => task.id === mode.task.id) ?? null;
      setBaseline(match);
      setConflictMessage(t(match ? "scheduledTasks.editor.versionConflict" : "scheduledTasks.editor.versionConflictRemoved"));
    }
  }

  return (
    <Sheet closeDisabled={pending} onClose={onClose} placement="right" title={title}>
      <div className="grid gap-4">
        <ScheduledTaskForm agents={agents} draft={draft} issue={issue} onChange={setDraft} weekdayNames={weekdayNames} />
        <ScheduledTaskReview agent={agents.find((agent) => agent.id === draft.agentId)} draft={draft} weekdayNames={weekdayNames} />
        {conflictMessage ? <p className="text-xs text-destructive" role="alert">{conflictMessage}</p> : <MutationStatus state={mutation} />}
        <div className="flex items-center justify-end gap-2 border-t border-border pt-3">
          <Button disabled={pending} onClick={onClose} type="button" variant="outline">{t("scheduledTasks.editor.cancel")}</Button>
          <Button disabled={pending || issue !== null || removed} onClick={() => void submit()} type="button">
            {pending ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : <Save className="h-3.5 w-3.5" aria-hidden="true" />}
            {mode.kind === "edit" ? t("scheduledTasks.editor.save") : t("scheduledTasks.create")}
          </Button>
        </div>
      </div>
    </Sheet>
  );
}
