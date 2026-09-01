import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcw, SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { orderByAgentPriority } from "../../lib/agent-display-order";
import { agentService } from "../../services/runtime-agent-client";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliLaunchScope } from "../../types/cli-parameter";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";
import { DraftActionBar } from "../../ui/forms/DraftActionBar";
import type { SettingsDraftGuard } from "../settings-page-types";
import type { SettingsPageId } from "../settings-pages";
import { PageHeader } from "../pages/page-parts";
import { CliParameterFieldGroups } from "./cli-parameter-field-groups";
import { CliParameterPreviewPanel } from "./cli-parameter-preview-panel";
import { CliParameterRail } from "./cli-parameter-rail";
import { CliParameterToolbar } from "./cli-parameter-toolbar";
import { useCliParameterDrafts } from "./use-cli-parameter-drafts";
import { useCliParameterPreview } from "./use-cli-parameter-preview";
import {
  asCliParameterServiceError,
  cliParameterErrorMessageKey,
  type CliParameterFilter,
} from "./view-model";

const profilesQueryKey = ["cli-parameter-profiles"] as const;
const emptyProfiles: CliParameterProfile[] = [];

export function CliParametersPage({
  onDraftStateChange,
  searchTerm,
  onNavigate,
}: {
  onDraftStateChange?: (guard: SettingsDraftGuard | null) => void;
  searchTerm: string;
  onNavigate?: (pageId: SettingsPageId) => void;
}) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const queryClient = useQueryClient();
  const [activeAgentId, setActiveAgentId] = useState<ManagedCliAgentId>("claude-code");
  const [scope, setScope] = useState<CliLaunchScope>("chat");
  const [filter, setFilter] = useState<CliParameterFilter>("all");
  const [localQuery, setLocalQuery] = useState("");
  const [notice, setNotice] = useState<string | null>(null);

  const profilesQuery = useQuery({
    queryKey: profilesQueryKey,
    queryFn: () => agentService.listCliParameterProfiles(),
  });
  const profiles = useMemo<readonly CliParameterProfile[]>(
    () => orderByAgentPriority(profilesQuery.data ?? emptyProfiles, (profile) => profile.agentId),
    [profilesQuery.data],
  );
  const drafts = useCliParameterDrafts(profiles);
  const activeProfile = profiles.find((profile) => profile.agentId === activeAgentId);
  const draft = drafts.draftFor(activeAgentId);
  const conflict = drafts.conflictFor(activeAgentId);
  // A blocking diagnostic is the server saying this profile is not in a saveable state.
  const blocked = (activeProfile?.diagnostics ?? []).some((entry) => entry.blocking);
  const activeDirtyCount = drafts.dirtyIdsFor(activeAgentId).length;

  const preview = useCliParameterPreview(
    activeProfile ? activeAgentId : null,
    activeProfile?.catalogVersion ?? "",
    scope,
    draft.selections,
    Boolean(activeProfile),
  );

  const saveMutation = useMutation({
    mutationFn: () =>
      agentService.saveCliParameterProfile({
        agentId: activeAgentId,
        expectedRevision: draft.baselineRevision,
        catalogVersion: draft.baselineCatalogVersion,
        selections: draft.selections,
      }),
    onSuccess: async (profile) => {
      drafts.accept(profile);
      setNotice(t("cliParameters.notice.saved"));
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  });
  const resetMutation = useMutation({
    mutationFn: () =>
      agentService.resetCliParameterProfile({
        agentId: activeAgentId,
        expectedRevision: draft.baselineRevision,
        catalogVersion: draft.baselineCatalogVersion,
      }),
    onSuccess: async (profile) => {
      drafts.accept(profile);
      setNotice(t("cliParameters.notice.reset"));
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  });

  /** Task 12.12: reports the active agent's own draft state so the shell can guard leaving
   *  Settings entirely (`onReturn`) -- the page already survives an *in-app* page switch on its
   *  own via `keepAlive: "draft-only"` (task 12.17), so this only covers the one departure that
   *  lifecycle can't. Scoped to the active agent, matching what `DraftActionBar` already shows on
   *  screen: a *different* agent's own dirty draft is not covered by this guard instance (only
   *  the header's cross-agent badge signals it), and the pre-existing `beforeunload` handler in
   *  `use-cli-parameter-drafts.ts` remains the only protection against losing it by closing the
   *  whole window. */
  useEffect(() => {
    if (!onDraftStateChange) return;
    if (activeDirtyCount === 0) { onDraftStateChange(null); return; }
    onDraftStateChange({
      canSave: drafts.canSaveFor(activeAgentId) && !blocked,
      dirtyCount: activeDirtyCount,
      discard: () => drafts.discard(activeAgentId),
      save: async () => { await saveMutation.mutateAsync(); },
    });
    return () => onDraftStateChange(null);
  }, [activeAgentId, activeDirtyCount, blocked, drafts, onDraftStateChange, saveMutation]);

  const query = (searchTerm || localQuery).trim().toLocaleLowerCase();
  const error =
    profilesQuery.error ?? saveMutation.error ?? resetMutation.error ?? preview.error ?? null;
  const structured = asCliParameterServiceError(error);
  const errorMessage = structured
    ? t(cliParameterErrorMessageKey(structured.code), {
        parameter: structured.parameterId ?? "",
        ...structured.details,
      })
    : error
      ? t("cliParameters.error.requestFailed", { message: String(error) })
      : null;

  async function selectAgent(next: ManagedCliAgentId) {
    // Drafts survive the switch, so the guard exists to warn, not to block.
    setNotice(null);
    setActiveAgentId(next);
  }

  async function discardActiveDraft() {
    if (
      !(await confirm({
        title: t("cliParameters.guard.title"),
        description: t("cliParameters.guard.body", {
          count: String(drafts.dirtyIdsFor(activeAgentId).length),
        }),
        tone: "danger",
      }))
    ) {
      return;
    }
    drafts.discard(activeAgentId);
  }

  async function restoreInherited() {
    if (!activeProfile) return;
    if (!(await confirm({ title: t("cliParameters.confirmReset"), tone: "danger" }))) return;
    drafts.inheritAll(
      activeAgentId,
      activeProfile.fields.map((field) => field.definition),
    );
  }

  return (
    <div className="space-y-4">
      {confirmationDialog}
      <PageHeader
        actions={
          <>
            {drafts.totalDirtyCount > 0 ? (
              <Badge tone="warning">
                {t("cliParameters.badge.dirty", { count: String(drafts.totalDirtyCount) })}
              </Badge>
            ) : null}
            <Button
              disabled={!activeProfile}
              onClick={() => void restoreInherited()}
              variant="destructive"
            >
              <RotateCcw aria-hidden="true" /> {t("cliParameters.actions.restoreInherited")}
            </Button>
          </>
        }
        description={t("cliParameters.description")}
        icon={SlidersHorizontal}
        title={t("cliParameters.title")}
      />

      {/* DraftActionBar is sticky, not layout-reserving -- it can only overlap content, never push
          it up. This bottom padding is how the page keeps its own controls clear of the bar once
          a save becomes available, and only while one actually is (no permanent empty space). */}
      <div className={`grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)] ${activeDirtyCount > 0 ? "pb-20" : ""}`}>
        <CliParameterRail
          activeAgentId={activeAgentId}
          dirtyCountFor={(agentId) => drafts.dirtyIdsFor(agentId).length}
          onOpenCliManagement={() => onNavigate?.("providers")}
          onSelect={(next) => void selectAgent(next)}
          profiles={profiles}
        />

        <div className="min-w-0 space-y-4">
          <CliParameterToolbar
            filter={filter}
            onFilterChange={setFilter}
            onQueryChange={setLocalQuery}
            onScopeChange={setScope}
            query={localQuery}
            scope={scope}
          />

          <div aria-live="polite" className="space-y-2">
            {conflict !== "none" ? (
              <div className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">
                <p className="font-medium">{t("cliParameters.conflict.title")}</p>
                <p className="mt-1">{t("cliParameters.conflict.body")}</p>
                <Button
                  className="mt-2"
                  disabled={!activeProfile}
                  onClick={() => activeProfile && drafts.reload(activeProfile)}
                  size="sm"
                  variant="outline"
                >
                  {t("cliParameters.actions.reload")}
                </Button>
              </div>
            ) : null}
            {errorMessage ? (
              <div className="rounded-md border p-3 text-sm ucd-status-danger" role="alert">
                {errorMessage}
              </div>
            ) : null}
            {notice ? (
              <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div>
            ) : null}
          </div>

          <p className="text-xs leading-5 text-muted-foreground">
            {t("cliParameters.policyPrecedenceNotice")}{" "}
            <Button
              className="h-auto px-1 py-0 align-baseline underline"
              onClick={() => onNavigate?.("agent-policies")}
              size="sm"
              variant="ghost"
            >
              {t("cliParameters.policyLink")}
            </Button>
          </p>

          {/* Two columns only once there is genuinely room for them. At 1440 the field rows already
              split into label and control, so carving out a third column there squeezes the
              descriptions into a vertical ribbon; below `2xl` the preview simply follows the
              controls, which is also what narrow widths need. */}
          <div className="grid min-w-0 gap-4 2xl:grid-cols-[minmax(0,1fr)_360px] 2xl:items-start">
            <div className="min-w-0">
              {activeProfile ? (
                <CliParameterFieldGroups
                  agentId={activeAgentId}
                  drafts={drafts}
                  filter={filter}
                  profile={activeProfile}
                  query={query}
                  scope={scope}
                />
              ) : null}
            </div>
            <div className="min-w-0 2xl:sticky 2xl:top-4">
              <CliParameterPreviewPanel
                refreshing={preview.refreshing}
                segments={preview.preview?.segments ?? null}
                stale={preview.stale}
              />
            </div>
          </div>
        </div>
      </div>

      <DraftActionBar
        dirtyCount={activeDirtyCount}
        onDiscard={() => void discardActiveDraft()}
        onSave={() => saveMutation.mutate()}
        pending={saveMutation.isPending}
        saveDisabled={!drafts.canSaveFor(activeAgentId) || blocked}
      />
    </div>
  );
}
