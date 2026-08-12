import { useState, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type { SkillOverlayDetail, SkillOverlayImportReview, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { SkillOverlayImportReviewPanel } from "./skill-overlay-import-review";
import { witnessesFor } from "./skill-overlay-mutation-dialog";
import { formatBytes } from "./skill-overlay-resource-preview";

const MAXIMUM_IMPORT_BYTES = 8 * 1_048_576;

interface SelectedArchive {
  sourceName: string;
  size: number;
  archive: number[];
}

export function SkillOverlayImportDialog({
  detail,
  target,
  returnFocus,
  onClose,
  onCommitted,
  onRefresh,
}: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const pinned = detail.summary.pinned;
  const [selected, setSelected] = useState<SelectedArchive | null>(null);
  const [review, setReview] = useState<SkillOverlayImportReview | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);
  const [pending, setPending] = useState<"reading" | "import" | "promote" | "reload" | null>(null);
  const initialWitnesses = witnessesFor(detail, target);
  const promotionWitnesses = review ? {
    ...initialWitnesses,
    expectedOverlayRevision: review.revision,
    expectedPayloadHash: review.documentHash,
  } : null;

  async function selectArchive(event: ChangeEvent<HTMLInputElement>) {
    if (pinned) return;
    const file = event.currentTarget.files?.[0];
    setReview(null);
    setAcknowledged(false);
    setError(null);
    if (!file) return setSelected(null);
    if (!file.name.toLowerCase().endsWith(".zip")) return rejectSelection(event, "skills.overlay.import.zipRequired");
    if (file.size === 0) return rejectSelection(event, "skills.overlay.import.emptyArchive");
    if (file.size > MAXIMUM_IMPORT_BYTES) return rejectSelection(event, "skills.overlay.import.tooLarge");
    setPending("reading");
    try {
      setSelected({ sourceName: file.name, size: file.size, archive: [...new Uint8Array(await file.arrayBuffer())] });
    } catch (caught) {
      setSelected(null);
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: false });
    } finally {
      setPending(null);
    }
  }

  function rejectSelection(event: ChangeEvent<HTMLInputElement>, key: string) {
    setSelected(null);
    event.currentTarget.value = "";
    setError({ message: t(key, { maximum: formatBytes(MAXIMUM_IMPORT_BYTES) }), stale: false });
  }

  async function importToQuarantine() {
    if (pinned || !selected) return;
    setPending("import");
    setError(null);
    try {
      const result = await agentService.importSkillOverlay({
        target,
        witnesses: initialWitnesses,
        sourceName: selected.sourceName,
        archive: selected.archive,
      });
      setReview(result);
      setAcknowledged(false);
      onCommitted();
    } catch (caught) {
      const normalized = normalizeSkillOverlayError(caught);
      setError({ message: normalized.message, stale: normalized.kind === "stale" });
    } finally {
      setPending(null);
    }
  }

  async function promote() {
    if (pinned || !review || !promotionWitnesses || !acknowledged || !review.scan.passed) return;
    setPending("promote");
    setError(null);
    try {
      await agentService.promoteSkillOverlay({
        target,
        witnesses: promotionWitnesses,
        reviewedRevision: review.revision,
        reviewedDocumentHash: review.documentHash,
        reviewedScan: review.scan,
      });
      onCommitted();
      onClose();
    } catch (caught) {
      const normalized = normalizeSkillOverlayError(caught);
      setError({ message: normalized.message, stale: normalized.kind === "stale" });
    } finally {
      setPending(null);
    }
  }

  async function reloadForReview() {
    setPending("reload");
    try {
      await onRefresh();
      setReview(null);
      setAcknowledged(false);
      setError(null);
    } catch (caught) {
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: true });
    } finally {
      setPending(null);
    }
  }

  const busy = pending !== null;
  return <ApplicationDialog closeDisabled={busy} description={t("skills.overlay.import.description")} maxWidth="max-w-4xl" onClose={onClose} returnFocus={returnFocus} title={t("skills.overlay.import.title")}>
    <div className="space-y-4">
      <div className="rounded-md border border-warning/40 bg-warning/10 p-3 text-xs leading-5">
        <p className="font-semibold">{t("skills.overlay.import.quarantineTitle")}</p>
        <p className="mt-1 text-muted-foreground">{t("skills.overlay.import.quarantineDescription")}</p>
      </div>
      {!review ? <>
        <label className="block text-sm">{t("skills.overlay.import.chooseArchive")}<span aria-hidden className="text-destructive"> *</span>
          <input accept=".zip,application/zip" className="mt-1 block min-h-11 w-full rounded-md border border-border bg-background px-3 py-2 text-sm file:mr-3 file:rounded file:border-0 file:bg-muted file:px-2 file:py-1" data-dialog-autofocus disabled={pinned} onChange={(event) => void selectArchive(event)} type="file" />
        </label>
        <p className="text-xs text-muted-foreground">{t("skills.overlay.import.limitHint", { maximum: formatBytes(MAXIMUM_IMPORT_BYTES) })}</p>
        {selected ? <dl className="grid gap-2 text-xs sm:grid-cols-2">
          <Metric label={t("skills.overlay.import.safeSource")} value={selected.sourceName} />
          <Metric label={t("skills.overlay.import.archiveSize")} value={formatBytes(selected.size)} />
        </dl> : null}
      </> : <>
        <SkillOverlayImportReviewPanel review={review} witnesses={promotionWitnesses} />
        <label className="flex min-h-11 items-start gap-3 rounded-md border border-border bg-background px-3 py-2 text-sm">
          <input checked={acknowledged} className="mt-0.5 h-4 w-4" disabled={pinned} onChange={(event) => setAcknowledged(event.target.checked)} type="checkbox" />
          <span>{t("skills.overlay.import.acknowledge")}</span>
        </label>
      </>}
      {error ? <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
        <p>{error.message}</p>
        {error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reloadForReview()} size="sm" variant="outline">{t("skills.overlay.import.reloadReview")}</Button> : null}
      </div> : null}
    </div>
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:items-center sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      {!review ? <Button disabled={pinned || busy || !selected} onClick={() => void importToQuarantine()}>{t("skills.overlay.import.importAction")}</Button> : null}
      {review ? <Button disabled={pinned || busy || !acknowledged || !review.scan.passed} onClick={() => void promote()}>{t("skills.overlay.import.promoteAction")}</Button> : null}
    </div>
  </ApplicationDialog>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-md border border-border bg-muted/10 px-3 py-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}
