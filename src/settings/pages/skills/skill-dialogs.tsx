import { useEffect, useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { Skill, SkillMetadata, SkillPreview, SkillScope, SkillSource } from "../../../types/skill";

type DialogMode = "create" | "edit" | "import" | "restore" | "delete" | null;
type EditTab = "edit" | "preview";
type PreviewTab = "rendered" | "source";
const emptyRestoreCandidates: string[] = [];

export interface SkillDialogState {
  mode: DialogMode;
  skill: Skill | null;
  preview: SkillPreview | null;
  editBody?: string;
}

export const closedSkillDialog: SkillDialogState = { mode: null, skill: null, preview: null };

export function SkillDialogs({
  state, scope, workspacePath, onClose, onCreate, onUpdate, onImport, onRestore, onDelete,
  onReloadEdit, restoreCandidates, editConflict, editError, operationError, operationPending, reloadingEdit,
}: {
  state: SkillDialogState;
  scope: SkillScope;
  workspacePath: string | null;
  onClose: () => void;
  onCreate: (metadata: SkillMetadata, body: string, source: SkillSource) => void;
  onUpdate: (skill: Skill, metadata: SkillMetadata, body: string) => void;
  onImport: (sourcePath: string) => void;
  onRestore?: (skillId: string) => void;
  onDelete?: (skill: Skill) => void;
  onReloadEdit: (skill: Skill) => void;
  restoreCandidates?: string[];
  editConflict: boolean;
  editError: string | null;
  operationError?: string | null;
  operationPending: boolean;
  reloadingEdit: boolean;
}) {
  const { t } = useTranslation();
  const [metadata, setMetadata] = useState<SkillMetadata>(emptyMetadata());
  const [body, setBody] = useState("");
  const [path, setPath] = useState("");
  const [restoreId, setRestoreId] = useState("");
  const [editTab, setEditTab] = useState<EditTab>("edit");
  const [previewTab, setPreviewTab] = useState<PreviewTab>("rendered");
  const candidates = restoreCandidates ?? emptyRestoreCandidates;

  useEffect(() => {
    if (state.mode === "edit" && state.skill) {
      setMetadata(state.skill.metadata);
      setBody(state.editBody ?? "");
      setEditTab("edit");
    } else if (state.mode === "create") {
      setMetadata(emptyMetadata());
      setBody("");
      setEditTab("edit");
    }
  }, [state.editBody, state.mode, state.skill]);

  useEffect(() => {
    if (state.mode === "restore" && !candidates.includes(restoreId)) setRestoreId(candidates[0] ?? "");
  }, [candidates, restoreId, state.mode]);

  useEffect(() => {
    if (state.preview) setPreviewTab("rendered");
  }, [state.preview]);

  if (state.preview) return <Dialog title={state.preview.id} onClose={onClose}>
    <p className="mb-3 truncate text-xs text-muted-foreground">{normalizeDisplayPath(state.preview.path)}</p>
    <div className="mb-3 flex gap-1 rounded-md bg-muted p-1" role="tablist">
      {(["rendered", "source"] as const).map((tab) => <button aria-selected={previewTab === tab} className={`flex-1 rounded px-3 py-2 text-sm ${previewTab === tab ? "bg-background font-semibold shadow-xs" : "text-muted-foreground"}`} key={tab} onClick={() => setPreviewTab(tab)} role="tab" type="button">{t(`skills.dialog.${tab}Tab`)}</button>)}
    </div>
    {previewTab === "rendered" ? <MarkdownPreview content={state.preview.content} /> : <MarkdownSource content={state.preview.content} />}
    <DialogActions onClose={onClose} />
  </Dialog>;

  if (!state.mode) return null;
  if (state.mode === "delete" && state.skill) return <Dialog closeDisabled={operationPending} title={t("skills.dialog.deleteTitle")} description={t("skills.dialog.deleteDescription", { name: state.skill.metadata.name })} onClose={onClose}>
    <OperationError message={operationError} />
    <DialogActions dangerLabel={t("skills.dialog.confirmDelete")} onClose={onClose} onSubmit={() => onDelete?.(state.skill!)} pending={operationPending} />
  </Dialog>;
  if (state.mode === "import") return <Dialog closeDisabled={operationPending} title={t("skills.dialog.importTitle")} onClose={onClose}>
    <input className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" data-dialog-autofocus onChange={(event) => setPath(normalizeDisplayPath(event.target.value))} placeholder={t("skills.dialog.externalDirectory")} value={path} />
    <OperationError message={operationError} />
    <DialogActions onClose={onClose} onSubmit={() => onImport(path)} pending={operationPending} submitLabel={t("skills.dialog.import")} />
  </Dialog>;
  if (state.mode === "restore") return <Dialog closeDisabled={operationPending} title={t("skills.dialog.restoreTitle")} onClose={onClose}>
    <select className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" data-dialog-autofocus onChange={(event) => setRestoreId(event.target.value)} value={restoreId}>{candidates.map((id) => <option key={id} value={id}>{id}</option>)}</select>
    <OperationError message={operationError} />
    <DialogActions disabled={candidates.length === 0} onClose={onClose} onSubmit={() => onRestore?.(restoreId)} pending={operationPending} submitLabel={t("skills.dialog.restore")} />
  </Dialog>;

  const editing = state.mode === "edit" && state.skill;
  const formPending = operationPending || reloadingEdit;
  return <Dialog closeDisabled={formPending} title={editing ? t("skills.dialog.editTitle") : t("skills.dialog.createTitle")} onClose={onClose}>
    <div className="mb-3 flex gap-1 rounded-md bg-muted p-1" role="tablist">
      {(["edit", "preview"] as const).map((tab) => <button aria-selected={editTab === tab} className={`flex-1 rounded px-3 py-2 text-sm ${editTab === tab ? "bg-background font-semibold shadow-xs" : "text-muted-foreground"}`} key={tab} onClick={() => setEditTab(tab)} role="tab" type="button">{t(`skills.dialog.${tab}Tab`)}</button>)}
    </div>
    {editTab === "edit" ? <>
      <div className="grid gap-x-3 md:grid-cols-2"><Field disabled={Boolean(editing)} label="ID" onChange={(value) => setMetadata((current) => ({ ...current, id: value }))} value={metadata.id} /><Field label={t("skills.dialog.name")} onChange={(value) => setMetadata((current) => ({ ...current, name: value }))} value={metadata.name} /><Field label={t("skills.dialog.category")} onChange={(value) => setMetadata((current) => ({ ...current, category: value }))} value={metadata.category} /><Field label={t("skills.dialog.version")} onChange={(value) => setMetadata((current) => ({ ...current, version: value }))} value={metadata.version} /></div>
      <Field label={t("skills.dialog.description")} onChange={(value) => setMetadata((current) => ({ ...current, description: value }))} value={metadata.description} />
      <Field label={t("skills.dialog.triggers")} onChange={(value) => setMetadata((current) => ({ ...current, triggers: value.split(",").map((item) => item.trim()).filter(Boolean) }))} value={metadata.triggers.join(", ")} />
      <label className="mt-3 block text-sm">{t("skills.dialog.body")}<textarea className="mt-1 min-h-40 w-full rounded-md border border-border bg-background px-3 py-2 text-sm" onChange={(event) => setBody(event.target.value)} value={body} /></label>
    </> : <MarkdownPreview content={`# ${metadata.name || metadata.id}\n\n${body}`} />}
    <p className="mt-2 text-xs text-muted-foreground">{t("skills.dialog.scope")}: {t(`skills.scope.${scope}`)}{workspacePath ? ` (${normalizeDisplayPath(workspacePath)})` : ""}</p>
    <OperationError message={editError ?? operationError}>{editing && editConflict ? <div className="mt-2 flex flex-wrap items-center justify-between gap-2"><p>{t("skills.dialog.editConflict")}</p><Button disabled={reloadingEdit} onClick={() => onReloadEdit(state.skill!)} variant="outline">{t(reloadingEdit ? "skills.dialog.reloading" : "skills.dialog.reload")}</Button></div> : null}</OperationError>
    <DialogActions onClose={onClose} onSubmit={() => editing ? onUpdate(state.skill!, metadata, body) : onCreate(metadata, body, "user")} pending={formPending} submitLabel={t("skills.dialog.save")} />
  </Dialog>;
}

function Dialog({ title, description, children, onClose, closeDisabled = false }: { title: string; description?: string; children: ReactNode; onClose: () => void; closeDisabled?: boolean }) {
  return <ApplicationDialog closeDisabled={closeDisabled} description={description} onClose={onClose} title={title}>{children}</ApplicationDialog>;
}

function DialogActions({ onClose, onSubmit, submitLabel, dangerLabel, disabled, pending = false }: { onClose: () => void; onSubmit?: () => void; submitLabel?: string; dangerLabel?: string; disabled?: boolean; pending?: boolean }) {
  const { t } = useTranslation();
  return <div className="mt-5 flex items-center justify-end gap-2">{pending ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}<Button disabled={pending} onClick={onClose} variant="outline">{onSubmit ? t("skills.dialog.cancel") : t("skills.dialog.close")}</Button>{onSubmit ? <Button className={dangerLabel ? "bg-destructive text-destructive-foreground hover:bg-destructive/90" : undefined} disabled={disabled || pending} onClick={onSubmit}>{dangerLabel ?? submitLabel}</Button> : null}</div>;
}

function MarkdownPreview({ content }: { content: string }) {
  return <div className="grid max-h-[55vh] max-w-none gap-3 overflow-auto rounded-md border border-border bg-muted/30 p-4 text-sm leading-6 [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_h1]:text-xl [&_h1]:font-semibold [&_h2]:text-lg [&_h2]:font-semibold [&_li]:ml-5 [&_li]:list-disc"><ReactMarkdown skipHtml>{content}</ReactMarkdown></div>;
}

function MarkdownSource({ content }: { content: string }) {
  return <pre className="max-h-[55vh] overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-muted/30 p-4 font-mono text-xs leading-5"><code>{content}</code></pre>;
}

function OperationError({ message, children }: { message?: string | null; children?: ReactNode }) {
  if (!message && !children) return null;
  return <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">{message ? <p>{message}</p> : null}{children}</div>;
}

function Field({ label, value, disabled, onChange }: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return <label className="mt-3 block text-sm">{label}<input className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm disabled:bg-muted" disabled={disabled} onChange={(event) => onChange(event.target.value)} value={value} /></label>;
}

function emptyMetadata(): SkillMetadata {
  return { id: "", name: "", description: "", category: "general", version: "1.0.0", triggers: [] };
}
