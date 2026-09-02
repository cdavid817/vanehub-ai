import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { createCustomCliConfigPayload } from "../../../config/cli-agent-provider-presets";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { CliConfigAgentId, CliConfigApplyResult, CliConfigProfile } from "../../../types/cli-agent-config";
import { CliConfigActionDialog, type CliConfigActionDecision, type CliConfigPendingAction } from "./cli-config-action-dialog";
import { CliConfigProfileDialog, type CliConfigProfileDraft } from "./cli-config-profile-dialog";
import { CliConfigProfileList } from "./cli-config-profile-list";
import { CliConfigStatusSummary } from "./cli-config-status-summary";
import { CliConfigToolbar } from "./cli-config-toolbar";

// Exported so `AgentConfigurationsPage` can re-subscribe to the same cache entry for its own
// page-level diagnostics summary without refetching -- the same "already-exported query key"
// mechanism `code-intelligence-page.tsx` uses to read `LspConfigurationSection`'s own cache.
export const agentGlobalConfigQueryKey = (agentId: CliConfigAgentId) => ["agents", "global-config", agentId] as const;

function draftFromProfile(profile: CliConfigProfile): CliConfigProfileDraft {
  return { agentId: profile.agentId, profile, preset: null, payload: structuredClone(profile.payload) };
}

export function AgentGlobalConfigPanel({ agentId, searchTerm = "", service = defaultAgentService }: {
  agentId: CliConfigAgentId;
  searchTerm?: string;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<CliConfigProfileDraft | null>(null);
  const [pendingAction, setPendingAction] = useState<CliConfigPendingAction | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [lastApplyResult, setLastApplyResult] = useState<CliConfigApplyResult | null>(null);
  const [profileSearch, setProfileSearch] = useState("");

  const overviewQuery = useQuery({
    queryKey: agentGlobalConfigQueryKey(agentId),
    queryFn: async () => {
      const [presets, profiles, status] = await Promise.all([
        service.listCliConfigPresets(agentId),
        service.listCliConfigProfiles(agentId),
        service.getCliConfigStatus(agentId),
      ]);
      return { presets, profiles, status };
    },
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: agentGlobalConfigQueryKey(agentId) });

  useEffect(() => {
    setDraft(null);
    setPendingAction(null);
    setNotice(null);
    setLastApplyResult(null);
    setProfileSearch("");
  }, [agentId]);

  const duplicateMutation = useMutation({
    mutationFn: (profile: CliConfigProfile) => service.duplicateCliConfigProfile(agentId, profile.id),
    onSuccess: async () => { setNotice(t("agents.globalConfig.duplicated")); await refresh(); },
  });
  const deleteMutation = useMutation({
    mutationFn: (profile: CliConfigProfile) => service.deleteCliConfigProfile({ agentId, profileId: profile.id, detachApplied: profile.appliedState !== "saved" }),
    onSuccess: async () => { setNotice(t("agents.globalConfig.deleted")); await refresh(); },
  });
  const importMutation = useMutation({
    mutationFn: (name: string) => service.importCliConfigProfile({ agentId, name }),
    onSuccess: async () => { setNotice(t("agents.globalConfig.imported")); await refresh(); },
  });
  const applyMutation = useMutation({
    mutationFn: ({ profile, decision }: { profile: CliConfigProfile; decision: CliConfigActionDecision }) => service.applyCliConfigProfile({
      agentId,
      profileId: profile.id,
      confirmAuthFileReplacement: decision.confirmAuthFileReplacement,
    }),
    onSuccess: async (result) => {
      setLastApplyResult(result);
      setNotice(result.status === "succeeded" ? result.simulated ? t("agents.globalConfig.simulatedSuccess") : t("agents.globalConfig.appliedSuccess") : null);
      await refresh();
    },
  });

  const busy = duplicateMutation.isPending || deleteMutation.isPending || importMutation.isPending || applyMutation.isPending;
  const operationError = [overviewQuery.error, duplicateMutation.error, deleteMutation.error, importMutation.error, applyMutation.error].find(Boolean);

  async function confirmAction(decision: CliConfigActionDecision) {
    if (!pendingAction) return;
    if (pendingAction.kind === "import") await importMutation.mutateAsync(decision.importName ?? "");
    else if (pendingAction.kind === "delete") await deleteMutation.mutateAsync(pendingAction.profile);
    else await applyMutation.mutateAsync({ profile: pendingAction.profile, decision });
    setPendingAction(null);
  }

  if (overviewQuery.isLoading) return <div className="flex min-h-48 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;

  return (
    <div className="space-y-4">
      <CliConfigToolbar
        busy={busy}
        onAdd={() => setDraft({ agentId, profile: null, preset: null, payload: createCustomCliConfigPayload(agentId) })}
        onImport={() => setPendingAction({ kind: "import" })}
        onRefresh={() => { void refresh(); }}
        onSearchChange={setProfileSearch}
        search={profileSearch}
      />
      {overviewQuery.data ? <CliConfigStatusSummary status={overviewQuery.data.status} /> : null}
      {overviewQuery.data ? (
        <CliConfigProfileList busy={busy} onApply={(profile) => setPendingAction({ kind: "apply", profile })} onDelete={(profile) => setPendingAction({ kind: "delete", profile })} onDuplicate={(profile) => void duplicateMutation.mutateAsync(profile).catch(() => undefined)} onEdit={(profile) => setDraft(draftFromProfile(profile))} onValidate={(profile) => service.validateCliConfigCredential({ agentId, profileId: profile.id })} presets={overviewQuery.data.presets} profiles={overviewQuery.data.profiles} searchTerms={[searchTerm, profileSearch]} />
      ) : null}
      {operationError ? <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{operationError instanceof Error ? operationError.message : String(operationError)}</p> : null}
      {notice ? <p className="rounded-md border p-3 text-sm ucd-status-success" role="status">{notice}</p> : null}
      {lastApplyResult ? <div className="rounded-md border border-border p-4 text-sm text-muted-foreground">{lastApplyResult.error ? <p className="ucd-status-warning" role="alert">{lastApplyResult.error}</p> : null}<p>{lastApplyResult.simulated ? t("agents.globalConfig.noPathsChanged") : t("agents.globalConfig.affectedPaths")}</p>{lastApplyResult.affectedPaths.map((path) => <p className="mt-1 break-all font-mono text-xs" key={path}>{path}</p>)}{lastApplyResult.warnings.map((warning) => <p className="mt-2" key={warning}>{warning}</p>)}</div> : null}
      {draft && overviewQuery.data ? <CliConfigProfileDialog draft={draft} onClose={() => setDraft(null)} onSaved={async () => { setDraft(null); setNotice(t("agents.globalConfig.saved")); await refresh(); }} presets={overviewQuery.data.presets} service={service} /> : null}
      {pendingAction && overviewQuery.data ? <CliConfigActionDialog action={pendingAction} driftState={overviewQuery.data.status.driftState} onClose={() => setPendingAction(null)} onConfirm={(decision) => void confirmAction(decision).catch(() => undefined)} pending={busy} /> : null}
    </div>
  );
}
