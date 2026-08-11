import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type {
  SkillOverlayDetail,
  SkillOverlayPreview,
  SkillOverlayTargetInput,
  SkillOverlayWitnesses,
} from "../../../types/skill-overlay";
import { SkillOverlayPreviewPanel } from "./skill-overlay-preview-panel";

export type OverlayDialogKind = "patch" | "guidance";

interface PreviewRecord {
  fingerprint: string;
  result: SkillOverlayPreview;
}

export function SkillOverlayMutationDialog({
  detail,
  kind,
  target,
  returnFocus,
  onClose,
  onCommitted,
  onRefresh,
}: {
  detail: SkillOverlayDetail;
  kind: OverlayDialogKind;
  target: SkillOverlayTargetInput;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const pinned = detail.summary.pinned;
  const [oldString, setOldString] = useState("");
  const [newString, setNewString] = useState("");
  const [guidance, setGuidance] = useState("");
  const [replaceAll, setReplaceAll] = useState(false);
  const [preview, setPreview] = useState<PreviewRecord | null>(null);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);
  const [pending, setPending] = useState<"preview" | "commit" | "reload" | null>(null);
  const witnesses = witnessesFor(detail, target);
  const fingerprint = JSON.stringify({ target, witnesses, kind, oldString, newString, replaceAll, guidance });
  const latestFingerprint = useRef(fingerprint);
  latestFingerprint.current = fingerprint;
  const currentPreview = preview?.fingerprint === fingerprint ? preview.result : null;
  const invalid = kind === "patch" ? oldString.length === 0 : guidance.trim().length === 0;
  const matchCount = kind === "patch" && currentPreview ? exactMatchCount(detail.effectiveInstructions.content, oldString) : null;

  async function requestPreview() {
    if (pinned) return;
    if (invalid) {
      setError({ message: t(kind === "patch" ? "skills.overlay.mutation.patchRequired" : "skills.overlay.mutation.guidanceRequired"), stale: false });
      return;
    }
    const requestedFingerprint = fingerprint;
    setPending("preview");
    setError(null);
    try {
      const result = await agentService.previewSkillOverlay({
        target,
        witnesses,
        mutation: kind === "patch"
          ? { kind: "exactPatch", oldString, newString, replaceAll }
          : { kind: "learnedGuidance", guidance },
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
    if (pinned || !currentPreview?.canCommit) return;
    setPending("commit");
    setError(null);
    try {
      if (kind === "patch") {
        await agentService.createSkillOverlayPatch({ target, witnesses: currentPreview.witnesses, oldString, newString, replaceAll });
      } else {
        await agentService.createSkillOverlayGuidance({ target, witnesses: currentPreview.witnesses, guidance });
      }
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
  return <ApplicationDialog
    closeDisabled={busy}
    description={t(`skills.overlay.mutation.${kind}Description`)}
    maxWidth="max-w-3xl"
    onClose={onClose}
    returnFocus={returnFocus}
    title={t(`skills.overlay.mutation.${kind}Title`)}
  >
    <div className="space-y-4">
      <p className="rounded-md border border-border bg-muted/20 p-3 text-xs leading-5 text-muted-foreground">
        {t("skills.overlay.mutation.scope", { scope: t(`skills.overlay.scope.${target.scope}`) })}
      </p>
      {kind === "patch" ? <PatchFields
        disabled={pinned}
        newString={newString}
        oldString={oldString}
        onNewStringChange={setNewString}
        onOldStringChange={setOldString}
        onReplaceAllChange={setReplaceAll}
        replaceAll={replaceAll}
      /> : <GuidanceField disabled={pinned} guidance={guidance} onChange={setGuidance} />}
      {error ? <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
        <p>{error.message}</p>
        {error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reload()} size="sm" variant="outline">
          {t(pending === "reload" ? "skills.overlay.mutation.reloading" : "skills.overlay.mutation.reload")}
        </Button> : null}
      </div> : null}
      {currentPreview ? <SkillOverlayPreviewPanel
        matchCount={matchCount}
        matchCountIncomplete={detail.effectiveInstructions.truncated}
        preview={currentPreview}
      /> : <p className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">
        {t("skills.overlay.mutation.previewRequired")}
      </p>}
    </div>
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:items-center sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      <Button disabled={pinned || busy || invalid} onClick={() => void requestPreview()} variant="outline">{t("skills.overlay.mutation.preview")}</Button>
      <Button disabled={pinned || busy || !currentPreview?.canCommit} onClick={() => void commit()}>{t("skills.overlay.mutation.commit")}</Button>
    </div>
  </ApplicationDialog>;
}

function PatchFields({ disabled, oldString, newString, replaceAll, onOldStringChange, onNewStringChange, onReplaceAllChange }: {
  disabled: boolean;
  oldString: string; newString: string; replaceAll: boolean;
  onOldStringChange: (value: string) => void; onNewStringChange: (value: string) => void; onReplaceAllChange: (value: boolean) => void;
}) {
  const { t } = useTranslation();
  return <fieldset className="space-y-3">
    <legend className="text-sm font-semibold">{t("skills.overlay.mutation.patchFields")}</legend>
    <TextArea autofocus disabled={disabled} label={t("skills.overlay.mutation.oldString")} onChange={onOldStringChange} required value={oldString} />
    <TextArea disabled={disabled} label={t("skills.overlay.mutation.newString")} onChange={onNewStringChange} value={newString} />
    <label className="flex min-h-11 items-center gap-3 rounded-md border border-border px-3 py-2 text-sm sm:min-h-9">
      <input checked={replaceAll} className="h-4 w-4" disabled={disabled} onChange={(event) => onReplaceAllChange(event.target.checked)} type="checkbox" />
      {t("skills.overlay.mutation.replaceAll")}
    </label>
  </fieldset>;
}

function GuidanceField({ disabled, guidance, onChange }: { disabled: boolean; guidance: string; onChange: (value: string) => void }) {
  const { t } = useTranslation();
  return <TextArea autofocus disabled={disabled} label={t("skills.overlay.mutation.guidance")} onChange={onChange} required value={guidance} />;
}

function TextArea({ label, value, required = false, autofocus = false, disabled = false, onChange }: {
  label: string; value: string; required?: boolean; autofocus?: boolean; disabled?: boolean; onChange: (value: string) => void;
}) {
  return <label className="block text-sm">{label}{required ? <span aria-hidden className="text-destructive"> *</span> : null}
    <textarea
      aria-required={required}
      className="mt-1 min-h-28 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      data-dialog-autofocus={autofocus ? "true" : undefined}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value)}
      value={value}
    />
  </label>;
}

export function witnessesFor(detail: SkillOverlayDetail, target: SkillOverlayTargetInput): SkillOverlayWitnesses {
  return {
    expectedOverlayRevision: detail.summary.scopes.find((scope) => scope.scope === target.scope)?.revision ?? null,
    expectedBaseInstructionHash: detail.summary.baseInstructionHash,
    expectedBasePackageHash: detail.summary.basePackageHash,
    expectedPayloadHash: null,
    expectedPinned: detail.summary.pinned,
  };
}

function exactMatchCount(content: string, target: string) {
  return target.length === 0 ? 0 : content.split(target).length - 1;
}
