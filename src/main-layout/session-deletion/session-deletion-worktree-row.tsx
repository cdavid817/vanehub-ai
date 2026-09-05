import { useState } from "react";
import { Copy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { DeletionPreviewWorktree } from "../../types/session-deletion";
import { blockerKey, KNOWN_BLOCKERS, type WorktreeChoiceState } from "./session-deletion-model";

/**
 * One worktree in the confirmation. The row is always shown, with its reasons, when cleanup is
 * disabled: hiding the directory would hide the fact that it is being kept.
 */
export function SessionDeletionWorktreeRow({
  choice,
  disabled,
  onAcknowledge,
  onToggle,
  worktree,
}: {
  choice: WorktreeChoiceState;
  disabled: boolean;
  onAcknowledge: (acknowledged: boolean) => void;
  onToggle: () => void;
  worktree: DeletionPreviewWorktree;
}) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const removable = worktree.allowedPolicies.includes("remove-safe");
  const displayPath = normalizeDisplayPath(worktree.displayPath);
  const blockers = worktree.blockers.map((code) => ({
    code,
    label: (KNOWN_BLOCKERS as readonly string[]).includes(code) ? t(blockerKey(code)) : t("sessionDeletion.blocker.generic", { code }),
  }));
  const checksComplete = worktree.checks === "complete";
  const changes = worktree.changes;
  const hasChanges = Boolean(changes && (changes.trackedModified + changes.staged + changes.conflicted + changes.untracked > 0));
  const ignored = worktree.ignored;

  async function copyPath() {
    try {
      await navigator.clipboard.writeText(displayPath);
      setCopied(true);
      setTimeout(() => setCopied(false), 1_500);
    } catch {
      setCopied(false);
    }
  }

  return (
    <section className="grid gap-2 rounded-md border border-border p-3 text-xs" data-testid="session-deletion-worktree" data-worktree-key={worktree.worktreeKey}>
      <div className="grid gap-1">
        <div className="flex min-w-0 items-start gap-2">
          <span className="shrink-0 text-muted-foreground">{t("sessionDeletion.worktree.directory")}</span>
          <code className="min-w-0 flex-1 break-all font-mono" data-testid="session-deletion-worktree-path" title={displayPath}>{displayPath}</code>
          <button
            aria-label={t("sessionDeletion.worktree.copyPath")}
            className="grid h-6 w-6 shrink-0 place-items-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            onClick={() => void copyPath()}
            title={copied ? t("sessionDeletion.worktree.copied") : t("sessionDeletion.worktree.copyPath")}
            type="button"
          >
            <Copy aria-hidden="true" className="h-3.5 w-3.5" />
          </button>
        </div>
        {worktree.branch ? (
          <div className="flex gap-2"><span className="text-muted-foreground">{t("sessionDeletion.worktree.branch")}</span><code className="font-mono">{worktree.branch}</code></div>
        ) : null}
        <div className="flex gap-2" data-testid="session-deletion-worktree-status">
          <span className="text-muted-foreground">{t("sessionDeletion.worktree.status")}</span>
          <span>
            {!checksComplete
              ? t("sessionDeletion.worktree.checksIncomplete")
              : hasChanges
                ? t("sessionDeletion.worktree.hasChanges")
                : removable
                  ? t("sessionDeletion.worktree.clean")
                  : t("sessionDeletion.worktree.unavailable")}
          </span>
        </div>
        {checksComplete ? (
          <div className="flex gap-2"><span className="text-muted-foreground">{t("sessionDeletion.worktree.references")}</span><span>{t("sessionDeletion.worktree.referenceCount", { count: worktree.externalReferences.length })}</span></div>
        ) : null}
        {worktree.externalReferences.length > 0 ? (
          <ul className="grid gap-0.5 pl-4 text-muted-foreground">
            {worktree.externalReferences.slice(0, 5).map((reference) => (
              <li key={`${reference.kind}:${reference.id}`}>{t(`sessionDeletion.reference.${reference.kind}`, { defaultValue: reference.kind })}: {reference.label}</li>
            ))}
          </ul>
        ) : null}
        {blockers.length > 0 ? (
          <ul className="grid gap-0.5 text-muted-foreground" data-testid="session-deletion-worktree-blockers">
            {blockers.map((blocker) => <li key={blocker.code}>{blocker.label}</li>)}
          </ul>
        ) : null}
      </div>
      <label className={`flex items-start gap-2 ${removable ? "" : "opacity-60"}`}>
        <input
          checked={choice.remove}
          className="mt-0.5 h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
          data-testid="session-deletion-remove-worktree"
          disabled={disabled || !removable}
          onChange={onToggle}
          type="checkbox"
        />
        <span className="grid gap-0.5">
          <span className="font-medium">{t("sessionDeletion.worktree.removeOption")}</span>
          <span className="text-muted-foreground">{t("sessionDeletion.worktree.removeDescription")}</span>
        </span>
      </label>
      {choice.remove && worktree.requiresIgnoredAcknowledgement && ignored ? (
        <div className="grid gap-2 rounded-md border border-destructive/40 p-2" data-testid="session-deletion-ignored">
          <p className="font-medium">{t("sessionDeletion.ignored.title", { count: ignored.totalEntries })}</p>
          <p className="text-muted-foreground">{t("sessionDeletion.ignored.warning")}</p>
          {ignored.completeness !== "complete" ? <p className="text-destructive">{t("sessionDeletion.ignored.incomplete")}</p> : null}
          {ignored.samples.length > 0 ? (
            <ul className="max-h-24 overflow-y-auto pl-4 font-mono text-muted-foreground">
              {ignored.samples.map((sample) => <li className="break-all" key={sample.path}>{sample.path}</li>)}
              {ignored.samplesTruncated ? <li>{t("sessionDeletion.ignored.truncated")}</li> : null}
            </ul>
          ) : null}
          <label className="flex items-start gap-2">
            <input
              checked={choice.acknowledgedFingerprint === ignored.fingerprint}
              className="mt-0.5 h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
              data-testid="session-deletion-acknowledge-ignored"
              disabled={disabled}
              onChange={(event) => onAcknowledge(event.target.checked)}
              type="checkbox"
            />
            <span>{t("sessionDeletion.ignored.acknowledge")}</span>
          </label>
        </div>
      ) : null}
    </section>
  );
}
