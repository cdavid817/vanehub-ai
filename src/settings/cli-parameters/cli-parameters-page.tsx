import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcw, Save, SlidersHorizontal, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { orderByAgentPriority } from "../../lib/agent-display-order";
import { agentService } from "../../services/runtime-agent-client";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliLaunchScope } from "../../types/cli-parameter";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";
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
  searchTerm,
  onNavigate,
}: {
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
              variant="outline"
            >
              <RotateCcw aria-hidden="true" /> {t("cliParameters.actions.restoreInherited")}
            </Button>
            <Button
              disabled={!drafts.isDirtyFor(activeAgentId)}
              onClick={() => void discardActiveDraft()}
              variant="outline"
            >
              <Undo2 aria-hidden="true" /> {t("cliParameters.actions.discardDraft")}
            </Button>
            <Button
              disabled={!drafts.canSaveFor(activeAgentId) || blocked || saveMutation.isPending}
              onClick={() => saveMutation.mutate()}
            >
              <Save aria-hidden="true" />{" "}
              {t(saveMutation.isPending ? "cliParameters.actions.saving" : "cliParameters.actions.save")}
            </Button>
          </>
        }
        description={t("cliParameters.description")}
        icon={SlidersHorizontal}
        title={t("cliParameters.title")}
      />

      <div className="grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)]">
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

          {/* Sticky at wide widths, and simply the last block at narrow ones, so the token
              list never needs a horizontal scrollbar to stay visible. */}
          <div className="lg:sticky lg:bottom-4">
            <CliParameterPreviewPanel
              refreshing={preview.refreshing}
              segments={preview.preview?.segments ?? null}
              stale={preview.stale}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
