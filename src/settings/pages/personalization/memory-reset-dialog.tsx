import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import {
  RESET_CONFIRMATION_PHRASE,
  type MaintenanceResult,
  type ResetScope,
} from "../../../types/personalization-memory";
import type { WorkspaceOption } from "./use-scope-options";

const SCOPES = ["any", "global", "workspace"] as const;

/**
 * Preview, then delete exactly what was previewed.
 *
 * The token comes from the preview and names the scope and statuses it counted, so the query key
 * below is the whole selection: changing any part of it fetches a new preview with a new token,
 * and there is no state in which the counts on screen and the token in hand describe different
 * things.
 */
export function MemoryResetDialog({
  onClose,
  service,
  workspaces,
}: {
  onClose: () => void;
  service: AgentService;
  workspaces: readonly WorkspaceOption[];
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [scope, setScope] = useState<ResetScope>({ includeArchived: false });
  const [typed, setTyped] = useState("");
  const [result, setResult] = useState<MaintenanceResult | null>(null);
  const [failed, setFailed] = useState(false);

  const needsWorkspace = scope.scopeKind === "workspace" && !scope.workspaceKey;

  const previewQuery = useQuery({
    queryKey: ["personalization", "reset-preview", scope] as const,
    queryFn: () => service.previewPersonalizationReset(scope),
    enabled: !needsWorkspace && result === null,
  });

  const executeMutation = useMutation({
    mutationFn: (token: string) =>
      service.executePersonalizationReset(scope, token, typed),
    onSuccess: (outcome) => {
      setResult(outcome);
      setFailed(false);
      void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "candidates"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
    },
    onError: () => setFailed(true),
  });

  function changeScope(next: Partial<ResetScope>) {
    // The typed phrase goes with the selection it was typed for. Keeping it would let a user
    // confirm one scope and delete another without retyping anything.
    setScope((current) => ({ ...current, ...next }));
    setTyped("");
    setFailed(false);
  }

  const preview = previewQuery.data;
  const phraseMatches = typed === RESET_CONFIRMATION_PHRASE;
  const blocked =
    !preview || needsWorkspace || !phraseMatches || executeMutation.isPending || preview.matched === 0;

  return (
    <ApplicationDialog
      description={t("personalization.reset.description")}
      onClose={onClose}
      title={t("personalization.reset.title")}
    >
      {result ? (
        <ResetResult result={result} />
      ) : (
        <div className="flex flex-col gap-4" data-testid="personalization-reset-form">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="flex flex-col gap-1 text-xs font-medium">
              {t("personalization.memoryList.filters.scope")}
              <select
                className="ucd-input h-9 rounded-md px-2 text-sm"
                data-testid="personalization-reset-scope"
                onChange={(event) =>
                  changeScope({
                    scopeKind: event.target.value === "any" ? undefined : (event.target.value as "global" | "workspace"),
                    workspaceKey:
                      event.target.value === "workspace" ? workspaces[0]?.workspaceKey : undefined,
                  })
                }
                value={scope.scopeKind ?? "any"}
              >
                {SCOPES.map((value) => (
                  <option key={value} value={value}>
                    {t(`personalization.memoryList.scope.${value}`)}
                  </option>
                ))}
              </select>
            </label>

            {scope.scopeKind === "workspace" ? (
              <label className="flex flex-col gap-1 text-xs font-medium">
                {t("personalization.scope.workspace")}
                <select
                  className="ucd-input h-9 rounded-md px-2 text-sm"
                  data-testid="personalization-reset-workspace"
                  onChange={(event) => changeScope({ workspaceKey: event.target.value || undefined })}
                  value={scope.workspaceKey ?? ""}
                >
                  <option value="">{t("personalization.scope.chooseWorkspace")}</option>
                  {workspaces.map((workspace) => (
                    <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
                      {workspace.displayName}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </div>

          <label className="flex items-center gap-2 text-xs font-medium">
            <input
              checked={scope.includeArchived}
              data-testid="personalization-reset-archived"
              onChange={(event) => changeScope({ includeArchived: event.target.checked })}
              type="checkbox"
            />
            {t("personalization.reset.includeArchived")}
          </label>

          {needsWorkspace ? (
            <p className="text-sm text-muted-foreground" data-testid="personalization-reset-needs-workspace">
              {t("personalization.scope.incomplete")}
            </p>
          ) : previewQuery.error ? (
            <p className="text-sm ucd-status-danger" data-testid="personalization-reset-preview-error" role="alert">
              {t("personalization.reset.previewFailed")}
            </p>
          ) : previewQuery.isPending || !preview ? (
            <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>
          ) : (
            <dl className="grid gap-2 sm:grid-cols-2" data-testid="personalization-reset-counts">
              <Count label={t("personalization.reset.count.matched")} value={preview.matched} />
              <Count label={t("personalization.reset.count.global")} value={preview.global} />
              <Count label={t("personalization.reset.count.workspace")} value={preview.workspace} />
              <Count label={t("personalization.reset.count.candidates")} value={preview.candidates} />
              {/* Counted because a reset removes them too, and a preview that omitted them would
                  understate what the user is about to lose. */}
              <Count label={t("personalization.reset.count.malformed")} value={preview.malformed} />
            </dl>
          )}

          {/* Said before the phrase, not after: there is no undo, and the files are plain markdown
              a user can copy anywhere. The application deliberately does not export for them --
              that would be a second copy of their memories it is then responsible for. */}
          <p className="text-xs text-muted-foreground" data-testid="personalization-reset-backup-hint">
            {t("personalization.reset.backupHint")}
          </p>

          <label className="flex flex-col gap-1 text-xs font-medium">
            {t("personalization.reset.typeToConfirm", { phrase: RESET_CONFIRMATION_PHRASE })}
            <input
              className="ucd-input h-9 rounded-md px-2 text-sm"
              data-dialog-autofocus
              data-testid="personalization-reset-phrase"
              onChange={(event) => setTyped(event.target.value)}
              value={typed}
            />
          </label>

          {failed ? (
            <p className="text-sm ucd-status-danger" data-testid="personalization-reset-failed" role="alert">
              {t("personalization.reset.failed")}
            </p>
          ) : null}

          <div className="flex flex-wrap gap-3">
            <Button
              data-testid="personalization-reset-execute"
              disabled={blocked}
              onClick={() => preview && executeMutation.mutate(preview.confirmationToken)}
            >
              {t("personalization.reset.execute")}
            </Button>
            <Button data-testid="personalization-reset-cancel" onClick={onClose} variant="outline">
              {t("personalization.detail.cancel")}
            </Button>
          </div>
        </div>
      )}
    </ApplicationDialog>
  );
}

function Count({ label, value }: { label: string; value: number }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="text-sm font-medium">{value}</dd>
    </div>
  );
}

/**
 * What actually happened, surface by surface.
 *
 * A partial result has to say so: a user told the reset succeeded while a projection row survived
 * would believe a memory is gone that a runtime can still recall.
 */
function ResetResult({ result }: { result: MaintenanceResult }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-3" data-testid="personalization-reset-result">
      <dl className="grid gap-2 sm:grid-cols-2">
        <Count label={t("personalization.reset.count.matched")} value={result.matched} />
        <Count label={t("personalization.reset.result.files")} value={result.deletedFiles} />
        <Count label={t("personalization.reset.result.projection")} value={result.removedProjectionRows} />
        <Count label={t("personalization.reset.result.retrieval")} value={result.revokedRetrievalEntries} />
        <Count label={t("personalization.reset.result.quarantined")} value={result.quarantined} />
      </dl>
      {result.failures.length > 0 ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning" data-testid="personalization-reset-partial" role="alert">
          <p>{t("personalization.reset.result.partial")}</p>
          <ul className="mt-1 list-disc pl-5 text-xs">
            {result.failures.map((phase) => (
              <li key={phase}>{t(`personalization.reset.phase.${phase}`)}</li>
            ))}
          </ul>
        </div>
      ) : (
        <p className="text-sm" data-testid="personalization-reset-complete">
          {t("personalization.reset.result.complete")}
        </p>
      )}
    </div>
  );
}
