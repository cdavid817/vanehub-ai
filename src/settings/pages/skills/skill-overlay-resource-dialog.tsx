import { useRef, useState, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type { SkillOverlayDetail, SkillOverlayPreview, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { witnessesFor } from "./skill-overlay-mutation-dialog";
import { SkillOverlayResourcePreview, formatBytes } from "./skill-overlay-resource-preview";

const MAXIMUM_FILE_BYTES = 1_048_576;
const prohibitedExtensions = /\.(?:bat|cmd|com|dll|exe|msi|ps1|py|sh|wasm)$/i;

interface SelectedResource {
  fileName: string;
  size: number;
  lastModified: number;
  mediaType: string;
  content: number[];
}

interface ResourcePreviewRecord {
  fingerprint: string;
  result: SkillOverlayPreview;
}

export function SkillOverlayResourceDialog({
  detail,
  target,
  initialPath,
  returnFocus,
  onClose,
  onCommitted,
  onRefresh,
}: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  initialPath?: string;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const pinned = detail.summary.pinned;
  const [logicalPath, setLogicalPath] = useState(initialPath ?? "");
  const [selected, setSelected] = useState<SelectedResource | null>(null);
  const [preview, setPreview] = useState<ResourcePreviewRecord | null>(null);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);
  const [pending, setPending] = useState<"reading" | "preview" | "commit" | "reload" | null>(null);
  const existing = detail.resources.find((resource) => resource.logicalPath === logicalPath && resource.effectiveScope === target.scope);
  const witnesses = { ...witnessesFor(detail, target), expectedPayloadHash: existing?.contentHash ?? null };
  const fingerprint = JSON.stringify({ target, witnesses, logicalPath, file: selected && [selected.fileName, selected.size, selected.lastModified, selected.mediaType] });
  const latestFingerprint = useRef(fingerprint);
  latestFingerprint.current = fingerprint;
  const currentPreview = preview?.fingerprint === fingerprint ? preview.result : null;

  async function selectFile(event: ChangeEvent<HTMLInputElement>) {
    if (pinned) return;
    const file = event.currentTarget.files?.[0];
    setPreview(null);
    setError(null);
    if (!file) return setSelected(null);
    const rejected = localFileRejection(file);
    if (rejected) {
      setSelected(null);
      event.currentTarget.value = "";
      setError({ message: t(rejected, { maximum: formatBytes(MAXIMUM_FILE_BYTES) }), stale: false });
      return;
    }
    setPending("reading");
    try {
      const mediaType = inferMediaType(file.name) || file.type;
      const content = [...new Uint8Array(await file.arrayBuffer())];
      setSelected({ fileName: file.name, size: file.size, lastModified: file.lastModified, mediaType, content });
      if (!initialPath) setLogicalPath(`${mediaType.startsWith("image/") ? "assets" : "references"}/${file.name}`);
    } catch (caught) {
      setSelected(null);
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: false });
    } finally {
      setPending(null);
    }
  }

  function validateDraft() {
    if (!selected) return t("skills.overlay.resource.fileRequired");
    if (!isAllowedPath(logicalPath)) return t("skills.overlay.resource.pathInvalid");
    if (prohibitedExtensions.test(logicalPath)) return t("skills.overlay.resource.executableRejected");
    if (!selected.mediaType) return t("skills.overlay.resource.mediaMissing");
    return null;
  }

  async function requestPreview() {
    if (pinned) return;
    const validation = validateDraft();
    if (validation || !selected) return setError({ message: validation ?? t("skills.overlay.resource.fileRequired"), stale: false });
    const requestedFingerprint = fingerprint;
    setPending("preview");
    setError(null);
    try {
      const result = await agentService.previewSkillOverlay({
        target,
        witnesses,
        mutation: { kind: "supportingFile", logicalPath, mediaType: selected.mediaType, content: selected.content },
      });
      if (latestFingerprint.current === requestedFingerprint) setPreview({ fingerprint: requestedFingerprint, result });
    } catch (caught) {
      const normalized = normalizeSkillOverlayError(caught);
      setError({ message: normalized.message, stale: normalized.kind === "stale" });
    } finally {
      setPending(null);
    }
  }

  async function commit() {
    if (pinned || !selected || !currentPreview?.canCommit) return;
    setPending("commit");
    setError(null);
    const input = { target, witnesses: currentPreview.witnesses, logicalPath, mediaType: selected.mediaType, content: selected.content };
    try {
      if (existing) await agentService.replaceSkillOverlayFile(input);
      else await agentService.addSkillOverlayFile(input);
      onCommitted();
      onClose();
    } catch (caught) {
      const normalized = normalizeSkillOverlayError(caught);
      setPreview(null);
      setError({ message: normalized.message, stale: normalized.kind === "stale" });
    } finally {
      setPending(null);
    }
  }

  async function reload() {
    setPending("reload");
    try {
      await onRefresh();
      setPreview(null);
      setError(null);
    } catch (caught) {
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: true });
    } finally {
      setPending(null);
    }
  }

  const busy = pending !== null;
  return <ApplicationDialog closeDisabled={busy} description={t("skills.overlay.resource.dialogDescription")} maxWidth="max-w-3xl" onClose={onClose} returnFocus={returnFocus} title={t(initialPath ? "skills.overlay.resource.replaceTitle" : "skills.overlay.resource.addTitle")}>
    <div className="space-y-4">
      <div className="rounded-md border border-border bg-muted/20 p-3 text-xs leading-5 text-muted-foreground">
        <p>{t("skills.overlay.resource.allowedDirectories")}</p>
        <p>{t("skills.overlay.resource.limitHint", { maximum: formatBytes(MAXIMUM_FILE_BYTES) })}</p>
      </div>
      <label className="block text-sm">{t("skills.overlay.resource.chooseFile")}<span aria-hidden className="text-destructive"> *</span>
        <input accept=".md,.txt,.json,.yaml,.yml,.toml,.csv,.png,.jpg,.jpeg,.gif,.webp" className="mt-1 block min-h-11 w-full rounded-md border border-border bg-background px-3 py-2 text-sm file:mr-3 file:rounded file:border-0 file:bg-muted file:px-2 file:py-1" data-dialog-autofocus disabled={pinned} onChange={(event) => void selectFile(event)} type="file" />
      </label>
      <label className="block text-sm">{t("skills.overlay.resource.path")}<span aria-hidden className="text-destructive"> *</span>
        <input aria-describedby="skill-overlay-resource-path-hint" className="mt-1 min-h-11 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm disabled:bg-muted" disabled={pinned || Boolean(initialPath)} onChange={(event) => setLogicalPath(event.target.value.replaceAll("\\", "/"))} value={logicalPath} />
      </label>
      <p className="text-xs text-muted-foreground" id="skill-overlay-resource-path-hint">{t("skills.overlay.resource.pathHint")}</p>
      {selected ? <dl className="grid gap-2 text-xs sm:grid-cols-2">
        <DraftMetric label={t("skills.overlay.resource.fileName")} value={selected.fileName} />
        <DraftMetric label={t("skills.overlay.resource.mediaType")} value={selected.mediaType} />
        <DraftMetric label={t("skills.overlay.resource.size")} value={formatBytes(selected.size)} />
        <DraftMetric label={t("skills.overlay.resource.operation")} value={t(existing ? "skills.overlay.resource.replace" : "skills.overlay.resource.add")} />
      </dl> : null}
      {error ? <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert"><p>{error.message}</p>{error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reload()} size="sm" variant="outline">{t(pending === "reload" ? "skills.overlay.mutation.reloading" : "skills.overlay.mutation.reload")}</Button> : null}</div> : null}
      {currentPreview && selected ? <SkillOverlayResourcePreview existing={existing} logicalPath={logicalPath} mediaType={selected.mediaType} preview={currentPreview} replacing={Boolean(existing)} sizeBytes={selected.size} /> : <p className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">{t("skills.overlay.mutation.previewRequired")}</p>}
    </div>
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:items-center sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      <Button disabled={pinned || busy || !selected} onClick={() => void requestPreview()} variant="outline">{t("skills.overlay.mutation.preview")}</Button>
      <Button disabled={pinned || busy || !currentPreview?.canCommit} onClick={() => void commit()}>{t(existing ? "skills.overlay.resource.replace" : "skills.overlay.resource.add")}</Button>
    </div>
  </ApplicationDialog>;
}

function DraftMetric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-md border border-border bg-muted/10 px-3 py-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}

function localFileRejection(file: File) {
  if (file.size > MAXIMUM_FILE_BYTES) return "skills.overlay.resource.fileTooLarge";
  if (prohibitedExtensions.test(file.name)) return "skills.overlay.resource.executableRejected";
  if (!inferMediaType(file.name) && !file.type) return "skills.overlay.resource.unsupportedMedia";
  return null;
}

function isAllowedPath(value: string) {
  const parts = value.split("/");
  return /^(references|templates|assets)\//.test(value)
    && value.length <= 240
    && parts.length <= 8
    && parts.every((part) => part.length > 0 && part !== ".." && !part.startsWith("."));
}

function inferMediaType(fileName: string) {
  const extension = fileName.split(".").pop()?.toLowerCase();
  return ({ md: "text/markdown", txt: "text/plain", json: "application/json", yaml: "application/yaml", yml: "application/yaml", toml: "application/toml", csv: "text/csv", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", webp: "image/webp" } as Record<string, string>)[extension ?? ""] ?? "";
}
