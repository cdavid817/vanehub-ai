import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { Skill, SkillMetadata, SkillPreview, SkillScope, SkillSource } from "../../../types/skill";

type DialogMode = "create" | "edit" | "import" | "restore" | null;

export interface SkillDialogState {
  mode: DialogMode;
  skill: Skill | null;
  preview: SkillPreview | null;
}

export function SkillDialogs({
  state,
  scope,
  workspacePath,
  onClose,
  onCreate,
  onUpdate,
  onImport,
  onRestore,
}: {
  state: SkillDialogState;
  scope: SkillScope;
  workspacePath: string | null;
  onClose: () => void;
  onCreate: (metadata: SkillMetadata, body: string, source: SkillSource) => void;
  onUpdate: (skill: Skill, metadata: SkillMetadata, body: string) => void;
  onImport: (sourcePath: string) => void;
  onRestore: (skillId: string) => void;
}) {
  const { t } = useTranslation();
  const [metadata, setMetadata] = useState<SkillMetadata>(emptyMetadata());
  const [body, setBody] = useState("");
  const [path, setPath] = useState("");
  const [restoreId, setRestoreId] = useState("tdd-discipline");

  useEffect(() => {
    if (state.mode === "edit" && state.skill) {
      setMetadata(state.skill.metadata);
      setBody("");
    } else if (state.mode === "create") {
      setMetadata(emptyMetadata());
      setBody("");
    }
  }, [state.mode, state.skill]);

  if (state.preview) {
    return (
      <Modal title={state.preview.id} onClose={onClose}>
        <p className="mb-2 truncate text-xs text-muted-foreground">{state.preview.path}</p>
        <pre className="max-h-[60vh] overflow-auto rounded-md bg-muted p-3 text-xs">{state.preview.content}</pre>
      </Modal>
    );
  }

  if (!state.mode) return null;

  if (state.mode === "import") {
    return (
      <Modal title={t("skills.dialog.importTitle")} onClose={onClose}>
        <input
          className="w-full rounded-md border border-border px-3 py-2 text-sm"
          onChange={(event) => setPath(event.target.value)}
          placeholder={t("skills.dialog.externalDirectory")}
          value={path}
        />
        <div className="mt-4 flex justify-end gap-2">
          <Button onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
          <Button onClick={() => onImport(path)}>{t("skills.dialog.import")}</Button>
        </div>
      </Modal>
    );
  }

  if (state.mode === "restore") {
    return (
      <Modal title={t("skills.dialog.restoreTitle")} onClose={onClose}>
        <select className="w-full rounded-md border border-border px-3 py-2 text-sm" onChange={(event) => setRestoreId(event.target.value)} value={restoreId}>
          {["tdd-discipline", "code-review", "code-security-scan", "api-doc-generation", "unit-test-generation", "readme-generation"].map((id) => (
            <option key={id} value={id}>{id}</option>
          ))}
        </select>
        <div className="mt-4 flex justify-end gap-2">
          <Button onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
          <Button onClick={() => onRestore(restoreId)}>{t("skills.dialog.restore")}</Button>
        </div>
      </Modal>
    );
  }

  const editing = state.mode === "edit" && state.skill;
  return (
    <Modal title={editing ? t("skills.dialog.editTitle") : t("skills.dialog.createTitle")} onClose={onClose}>
      <div className="grid gap-3 md:grid-cols-2">
        <Field disabled={Boolean(editing)} label="ID" onChange={(value) => setMetadata((current) => ({ ...current, id: value }))} value={metadata.id} />
        <Field label={t("skills.dialog.name")} onChange={(value) => setMetadata((current) => ({ ...current, name: value }))} value={metadata.name} />
        <Field label={t("skills.dialog.category")} onChange={(value) => setMetadata((current) => ({ ...current, category: value }))} value={metadata.category} />
        <Field label={t("skills.dialog.version")} onChange={(value) => setMetadata((current) => ({ ...current, version: value }))} value={metadata.version} />
      </div>
      <Field label={t("skills.dialog.description")} onChange={(value) => setMetadata((current) => ({ ...current, description: value }))} value={metadata.description} />
      <Field label={t("skills.dialog.triggers")} onChange={(value) => setMetadata((current) => ({ ...current, triggers: value.split(",").map((item) => item.trim()).filter(Boolean) }))} value={metadata.triggers.join(", ")} />
      <label className="mt-3 block text-sm">
        {t("skills.dialog.body")}
        <textarea className="mt-1 min-h-32 w-full rounded-md border border-border px-3 py-2 text-sm" onChange={(event) => setBody(event.target.value)} value={body} />
      </label>
      <p className="mt-2 text-xs text-muted-foreground">
        {t("skills.dialog.scope")}: {t(`skills.scope.${scope}`)}{workspacePath ? ` (${workspacePath})` : ""}
      </p>
      <div className="mt-4 flex justify-end gap-2">
        <Button onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
        <Button onClick={() => editing ? onUpdate(state.skill!, metadata, body) : onCreate(metadata, body, "user")}>
          {t("skills.dialog.save")}
        </Button>
      </div>
    </Modal>
  );
}

function Field({ label, value, disabled, onChange }: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return (
    <label className="mt-3 block text-sm">
      {label}
      <input className="mt-1 w-full rounded-md border border-border px-3 py-2 text-sm disabled:bg-muted" disabled={disabled} onChange={(event) => onChange(event.target.value)} value={value} />
    </label>
  );
}

function Modal({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  const { t } = useTranslation();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <section className="max-h-[90vh] w-full max-w-2xl overflow-auto rounded-lg bg-background p-5 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="text-lg font-semibold">{title}</h3>
          <button className="text-sm text-muted-foreground" onClick={onClose} type="button">{t("skills.dialog.close")}</button>
        </div>
        {children}
      </section>
    </div>
  );
}

function emptyMetadata(): SkillMetadata {
  return { id: "", name: "", description: "", category: "general", version: "1.0.0", triggers: [] };
}
