import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { PromptHook, PromptHookMutationInput, PromptHookPreview } from "../../../types/prompt-hook";

type DialogMode = "create" | "edit" | "delete" | null;

export interface PromptHookDialogState {
  mode: DialogMode;
  hook: PromptHook | null;
  preview: PromptHookPreview | null;
}

export function PromptHookDialogs({
  state,
  onClose,
  onCreate,
  onUpdate,
  onDelete,
  error,
}: {
  state: PromptHookDialogState;
  onClose: () => void;
  onCreate: (input: PromptHookMutationInput) => void;
  onUpdate: (hook: PromptHook, input: PromptHookMutationInput) => void;
  onDelete: (hook: PromptHook) => void;
  error: string | null;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<PromptHookMutationInput>(emptyDraft());

  useEffect(() => {
    if (state.mode === "edit" && state.hook) {
      setDraft({ ...state.hook, templateBody: state.hook.templateBody ?? "" });
    } else if (state.mode === "create") {
      setDraft(emptyDraft());
    }
  }, [state.hook, state.mode]);

  if (state.preview) {
    return (
      <Modal title={state.preview.hookId ?? t("promptHooks.preview.assemblyTitle")} onClose={onClose}>
        <div className="mb-3 grid gap-2 text-xs text-muted-foreground md:grid-cols-3">
          {state.preview.trace.map((trace) => (
            <div className="rounded-md border border-border p-2" key={trace.id}>
              <div className="truncate font-mono text-foreground">{trace.hookId}</div>
              <div>{t(`promptHooks.status.${trace.status}`)}</div>
              <div>{trace.contentHash ?? "-"}</div>
              <div>{trace.tokenEstimate ?? 0}</div>
            </div>
          ))}
        </div>
        <pre className="max-h-[60vh] overflow-auto rounded-md bg-muted p-3 text-xs">{state.preview.renderedContent}</pre>
      </Modal>
    );
  }

  if (!state.mode) return null;

  if (state.mode === "delete" && state.hook) {
    return (
      <Modal title={t("promptHooks.dialog.deleteTitle")} onClose={onClose}>
        <p className="text-sm text-muted-foreground">{t("promptHooks.dialog.deleteDescription", { id: state.hook.id })}</p>
        <DialogError error={error} />
        <div className="mt-4 flex justify-end gap-2">
          <Button onClick={onClose} variant="outline">{t("promptHooks.dialog.cancel")}</Button>
          <Button onClick={() => onDelete(state.hook!)} variant="default">{t("promptHooks.dialog.delete")}</Button>
        </div>
      </Modal>
    );
  }

  const editing = state.mode === "edit" && state.hook;
  return (
    <Modal title={editing ? t("promptHooks.dialog.editTitle") : t("promptHooks.dialog.createTitle")} onClose={onClose}>
      <div className="grid gap-3 md:grid-cols-2">
        <Field disabled={Boolean(editing)} label="ID" onChange={(value) => setDraft((current) => ({ ...current, id: value }))} value={draft.id} />
        <Field label={t("promptHooks.dialog.name")} onChange={(value) => setDraft((current) => ({ ...current, name: value }))} value={draft.name} />
        <Select label={t("promptHooks.dialog.category")} onChange={(value) => setDraft((current) => ({ ...current, category: value as PromptHookMutationInput["category"] }))} value={draft.category} values={["bootstrap", "callback", "dynamic", "law", "navigation", "routing", "static"]} />
        <Select label={t("promptHooks.dialog.stage")} onChange={(value) => setDraft((current) => ({ ...current, stage: value as PromptHookMutationInput["stage"] }))} value={draft.stage} values={["session-init", "per-turn"]} />
      </div>
      <Field label={t("promptHooks.dialog.description")} onChange={(value) => setDraft((current) => ({ ...current, description: value }))} value={draft.description} />
      <label className="mt-3 block text-sm">
        {t("promptHooks.dialog.order")}
        <input className="mt-1 w-full rounded-md border border-border px-3 py-2 text-sm" onChange={(event) => setDraft((current) => ({ ...current, order: Number(event.target.value) }))} type="number" value={draft.order} />
      </label>
      <label className="mt-3 block text-sm">
        {t("promptHooks.dialog.body")}
        <textarea className="mt-1 min-h-40 w-full rounded-md border border-border px-3 py-2 font-mono text-sm" onChange={(event) => setDraft((current) => ({ ...current, templateBody: event.target.value }))} value={draft.templateBody} />
      </label>
      <label className="mt-3 flex items-center gap-2 text-sm">
        <input checked={draft.enabled} onChange={(event) => setDraft((current) => ({ ...current, enabled: event.target.checked }))} type="checkbox" />
        {t("promptHooks.enabled")}
      </label>
      <DialogError error={error} />
      <div className="mt-4 flex justify-end gap-2">
        <Button onClick={onClose} variant="outline">{t("promptHooks.dialog.cancel")}</Button>
        <Button onClick={() => editing ? onUpdate(state.hook!, draft) : onCreate(draft)}>{t("promptHooks.dialog.save")}</Button>
      </div>
    </Modal>
  );
}

function DialogError({ error }: { error: string | null }) {
  const { t } = useTranslation();
  if (!error) return null;

  return (
    <div className="mt-3 rounded-md border px-3 py-2 text-sm ucd-status-danger">
      <span className="font-medium">{t("promptHooks.dialog.errorTitle")}: </span>
      {t(localizeErrorKey(error))}
    </div>
  );
}

function localizeErrorKey(error: string) {
  if (error.includes("Invalid Prompt Hook id")) return "promptHooks.error.invalidId";
  if (error.includes("name is required")) return "promptHooks.error.nameRequired";
  if (error.includes("Unsupported Prompt Hook category")) return "promptHooks.error.unsupportedCategory";
  if (error.includes("Unsupported Prompt Hook stage")) return "promptHooks.error.unsupportedStage";
  if (error.includes("Invalid Prompt Hook order")) return "promptHooks.error.invalidOrder";
  if (error.includes("control characters")) return "promptHooks.error.controlCharacters";
  if (error.includes("Unsupported Prompt Hook CLI binding")) return "promptHooks.error.unsupportedCli";
  if (error.includes("Built-in Prompt Hook content cannot be edited")) return "promptHooks.error.builtinEdit";
  if (error.includes("Built-in Prompt Hook cannot be deleted")) return "promptHooks.error.builtinDelete";
  if (error.includes("Prompt Hook cannot be disabled")) return "promptHooks.error.immutableDisable";
  return "promptHooks.error.generic";
}

function Field({ label, value, disabled, onChange }: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return (
    <label className="mt-3 block text-sm">
      {label}
      <input className="mt-1 w-full rounded-md border border-border px-3 py-2 text-sm disabled:bg-muted" disabled={disabled} onChange={(event) => onChange(event.target.value)} value={value} />
    </label>
  );
}

function Select({ label, value, values, onChange }: { label: string; value: string; values: string[]; onChange: (value: string) => void }) {
  const { t } = useTranslation();
  return (
    <label className="mt-3 block text-sm">
      {label}
      <select className="mt-1 w-full rounded-md border border-border px-3 py-2 text-sm" onChange={(event) => onChange(event.target.value)} value={value}>
        {values.map((item) => (
          <option key={item} value={item}>{t(`promptHooks.option.${item}`)}</option>
        ))}
      </select>
    </label>
  );
}

function Modal({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <section className="max-h-[90vh] w-full max-w-2xl overflow-auto rounded-lg bg-background p-5 shadow-xl">
        <div className="mb-4 flex items-center justify-between gap-3">
          <h3 className="min-w-0 truncate text-lg font-semibold">{title}</h3>
          <button className="shrink-0 text-sm text-muted-foreground" onClick={onClose} type="button">{t("promptHooks.dialog.close")}</button>
        </div>
        {children}
      </section>
    </div>
  );
}

function emptyDraft(): PromptHookMutationInput {
  return {
    id: "",
    name: "",
    description: "",
    category: "dynamic",
    stage: "per-turn",
    order: 500,
    templateBody: "",
    enabled: true,
    cliBindings: [],
    governance: { safetyTier: "editable", transparencyTier: "opt-in-view", governanceTier: "human-gated" },
  };
}
