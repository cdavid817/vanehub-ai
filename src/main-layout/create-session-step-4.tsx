import { ClipboardCheck, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CreateSessionSection } from "./create-session-section";
import type { CreateSessionWizardStep } from "./create-session-wizard-steps";
import type { CreateSessionDraft } from "./create-session-draft-model";
import type { CreateSessionValidation, CreateSessionValidationReason } from "./create-session-validation";
import type { AgentRegistryEntry } from "../types/agent";

/** Task 11.10: which step owns each of `CreateSessionValidation`'s named slots, for the
 *  Review-level summary's "jump to the owning step" links. `sshConnection` is excluded --
 *  it is already shown inline by `RemoteWorkspaceSection` at the field it describes, and
 *  is a raw i18n key already, not a `CreateSessionValidationReason` code. */
const VALIDATION_FIELD_STEP: Record<"agent" | "seats" | "workspace", CreateSessionWizardStep> = {
  agent: 2,
  seats: 2,
  workspace: 3,
};

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(6rem,0.35fr)_1fr] gap-2 text-xs">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 wrap-break-word font-medium text-foreground">{value}</dd>
    </div>
  );
}

/**
 * Step 4 / Review (task 11.7): "runtime, participant, workspace, override, risk, and
 * resource-consequence summary." Runtime/participant/workspace are direct restatements of Steps
 * 1-3's own choices — nothing new to compute. The other three needed an honest interpretation,
 * since none names a field that already exists in `CreateSessionDraft`:
 * - "override": not just a passive summary line — the session name (auto-derived from the
 *   workspace unless the reader types their own, `titleUserEdited`) is genuinely last-mile
 *   editable right here, which is what "override" means for the one field a reader would
 *   plausibly still want to change after seeing the full picture. This also closes a real gap the
 *   old single-screen dialog's own separate "Identity" section covered and no other step in this
 *   one did — there was nowhere left to name a session before this fix.
 * - "risk": no policy/permission field exists at session-creation time at all (that is a
 *   post-creation, Settings-level concept — see task 10.18's own `effectiveExecutionPolicy`,
 *   which this draft has no equivalent of). The one concrete, defensible risk-adjacent fact this
 *   draft actually knows is that a remote workspace means executing commands on another host —
 *   shown as a plain, non-alarming note, not invented urgency.
 * - "resource-consequence": stated as the literal side effects submission will cause that this
 *   draft already knows about (a Git worktree gets created, an SSH connection gets saved, N Agent
 *   processes start for N seats) rather than a generic disclaimer.
 */
export function CreateSessionStep4({
  draft,
  effectivePersonalizationMode,
  onGoToStep,
  onTitleChange,
  selectedAgent,
  validation,
}: {
  draft: CreateSessionDraft;
  effectivePersonalizationMode: string;
  onGoToStep: (step: CreateSessionWizardStep) => void;
  onTitleChange: (value: string) => void;
  selectedAgent: AgentRegistryEntry | null;
  validation: CreateSessionValidation;
}) {
  const { t } = useTranslation();
  const seatCount = draft.agentMode === "multi" ? draft.multiSeats.length : 1;
  const consequences: string[] = [];
  if (draft.workspaceMode === "local" && draft.worktreeEnabled) consequences.push(t("createSession.review.consequenceWorktree", { name: draft.worktreeName }));
  if (draft.workspaceMode === "remote" && draft.saveSshConnection) consequences.push(t("createSession.review.consequenceSshSaved"));
  consequences.push(t("createSession.review.consequenceSeats", { count: seatCount }));

  const errors = (["agent", "seats", "workspace"] as const)
    .map((field) => ({ field, reason: validation[field] }))
    .filter((entry): entry is { field: "agent" | "seats" | "workspace"; reason: CreateSessionValidationReason } => entry.reason !== null);

  return (
    <CreateSessionSection hint={t("createSession.review.hint")} icon={ClipboardCheck} title={t("createSession.review.title")}>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("createSession.sessionName")}</span>
        <input
          className="ucd-input h-9 rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onChange={(event) => onTitleChange(event.target.value)}
          placeholder={t("createSession.sessionPlaceholder")}
          value={draft.title}
        />
      </label>
      {errors.length > 0 ? (
        <div className="grid gap-1.5 rounded-md border border-destructive/40 bg-destructive/5 p-2.5" role="alert">
          <p className="text-xs font-medium text-destructive">{t("createSession.review.errorsTitle")}</p>
          <ul className="grid gap-1">
            {errors.map(({ field, reason }) => (
              <li className="flex items-center justify-between gap-2 text-xs text-destructive" key={field}>
                <span>{t(`createSession.validation.${reason}`)}</span>
                <button
                  className="shrink-0 font-medium underline underline-offset-2 hover:no-underline"
                  onClick={() => onGoToStep(VALIDATION_FIELD_STEP[field])}
                  type="button"
                >
                  {t("createSession.review.fix")}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      <dl className="grid gap-2">
        <SummaryRow label={t("createSession.review.runtime")} value={t(`createSession.agentMode.${draft.agentMode}`)} />
        <SummaryRow
          label={t("createSession.review.participant")}
          value={draft.agentMode === "multi" ? t("createSession.review.seatCount", { count: seatCount }) : selectedAgent?.displayName ?? t("createSession.review.noAgent")}
        />
        <SummaryRow
          label={t("createSession.review.workspace")}
          value={draft.workspaceMode === "local" ? (draft.projectPath || t("createSession.review.noPath")) : (draft.remotePath ? `${draft.remoteHost}:${draft.remotePath}` : t("createSession.review.noPath"))}
        />
        <SummaryRow label={t("createSession.review.personalization")} value={t(`personalization.preview.modeValue.${effectivePersonalizationMode}`)} />
      </dl>
      {draft.workspaceMode === "remote" ? (
        <p className="flex items-start gap-1.5 text-xs text-muted-foreground">
          <TriangleAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          {t("createSession.review.remoteRisk")}
        </p>
      ) : null}
      <ul className="grid gap-1 text-xs text-muted-foreground">
        {consequences.map((consequence) => <li key={consequence}>{consequence}</li>)}
      </ul>
    </CreateSessionSection>
  );
}
