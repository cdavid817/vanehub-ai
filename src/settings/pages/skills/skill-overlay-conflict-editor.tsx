import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayDetail } from "../../../types/skill-overlay";
import type { SkillOverlayReconciliationConflictChoice } from "../../../types/skill-overlay-reconciliation";

interface ConflictDraft {
  resolution: "editPatch" | "ignore" | null;
  oldString: string;
  newString: string;
  replaceAll: boolean;
}

export type ConflictDrafts = Record<string, ConflictDraft>;

export function SkillOverlayConflictEditor({ conflicts, detail, disabled = false, drafts, onChange }: {
  conflicts: SkillOverlayReconciliationConflictChoice[];
  detail: SkillOverlayDetail;
  disabled?: boolean;
  drafts: ConflictDrafts;
  onChange: (value: ConflictDrafts) => void;
}) {
  const { t } = useTranslation();
  function update(conflictId: string, patch: Partial<ConflictDraft>) {
    onChange({ ...drafts, [conflictId]: { ...emptyDraft(), ...drafts[conflictId], ...patch } });
  }
  return <section aria-labelledby="skill-overlay-conflict-editor">
    <h4 className="text-sm font-semibold" id="skill-overlay-conflict-editor">{t("skills.overlay.reconcile.conflictsTitle")}</h4>
    <p className="mt-1 text-xs text-muted-foreground">{t("skills.overlay.reconcile.conflictsDescription")}</p>
    <div className="mt-3 space-y-3">
      {conflicts.map(({ conflict }) => {
        const draft = drafts[conflict.id] ?? emptyDraft();
        const mutation = detail.mutations.find((value) => value.id === conflict.mutationId);
        const editable = mutation?.kind === "patch";
        return <fieldset className="rounded-md border border-border bg-muted/10 p-3" disabled={disabled} key={conflict.id}>
          <legend className="px-1 text-xs font-semibold">{t("skills.overlay.reconcile.conflictLabel", { id: conflict.id })}</legend>
          <div className="flex flex-wrap items-center gap-2 text-xs"><Badge tone="warning">{conflict.safeReason}</Badge><span className="text-muted-foreground">{t("skills.overlay.reconcile.mutation", { id: conflict.mutationId })}</span></div>
          <div className="mt-3 grid gap-2 sm:grid-cols-2">
            {editable ? <ResolutionOption checked={draft.resolution === "editPatch"} label={t("skills.overlay.reconcile.editPatch")} name={conflict.id} onChange={() => update(conflict.id, { resolution: "editPatch" })} value="editPatch" /> : null}
            <ResolutionOption checked={draft.resolution === "ignore"} label={t("skills.overlay.reconcile.ignore")} name={conflict.id} onChange={() => update(conflict.id, { resolution: "ignore" })} value="ignore" />
          </div>
          {draft.resolution === "ignore" ? <p className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-3 text-xs leading-5">{t("skills.overlay.reconcile.ignoreHint")}</p> : null}
          {draft.resolution === "editPatch" ? <div className="mt-3 space-y-3">
            <TextArea label={t("skills.overlay.reconcile.oldString")} onChange={(oldString) => update(conflict.id, { oldString })} required value={draft.oldString} />
            <TextArea label={t("skills.overlay.reconcile.newString")} onChange={(newString) => update(conflict.id, { newString })} value={draft.newString} />
            <label className="flex min-h-11 items-center gap-3 rounded-md border border-border bg-background px-3 py-2 text-sm"><input checked={draft.replaceAll} className="h-4 w-4" onChange={(event) => update(conflict.id, { replaceAll: event.target.checked })} type="checkbox" />{t("skills.overlay.mutation.replaceAll")}</label>
          </div> : null}
        </fieldset>;
      })}
    </div>
  </section>;
}

function ResolutionOption({ checked, label, name, value, onChange }: { checked: boolean; label: string; name: string; value: string; onChange: () => void }) {
  return <label className="flex min-h-11 items-center gap-3 rounded-md border border-border bg-background px-3 py-2 text-sm"><input checked={checked} name={`resolution-${name}`} onChange={onChange} type="radio" value={value} />{label}</label>;
}

function TextArea({ label, value, required = false, onChange }: { label: string; value: string; required?: boolean; onChange: (value: string) => void }) {
  return <label className="block text-sm">{label}{required ? <span aria-hidden className="text-destructive"> *</span> : null}<textarea aria-required={required} className="mt-1 min-h-24 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => onChange(event.target.value)} value={value} /></label>;
}

function emptyDraft(): ConflictDraft {
  return { resolution: null, oldString: "", newString: "", replaceAll: false };
}
