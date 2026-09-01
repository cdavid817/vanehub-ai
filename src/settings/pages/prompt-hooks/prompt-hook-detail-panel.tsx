import { Trash2 } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { useConfirmation } from "../../../components/ui/use-confirmation";
import type { AgentService } from "../../../services/agent-service";
import type { ManagedCliAgentId } from "../../../types/agent";
import type { PromptHook, PromptHookMutationInput } from "../../../types/prompt-hook";
import { PromptHookContentEditor, PromptHookOverview } from "./prompt-hook-detail-sections";
import { localizePromptHookErrorKey } from "./prompt-hook-dialogs";
import { PromptHookVersionHistoryView } from "./prompt-hook-version-history";

type DetailTab = "overview" | "content" | "history";

export function PromptHookDetailPanel({
  agents,
  hook,
  service,
  onChanged,
  onClose,
  onDelete,
  onPreview,
  onToggleAgent,
  onToggleEnabled,
}: {
  agents: { id: ManagedCliAgentId; displayName: string }[];
  hook: PromptHook;
  service: AgentService;
  onChanged: () => void;
  onClose: () => void;
  onDelete: (hook: PromptHook) => void;
  onPreview: (hook: PromptHook) => void;
  onToggleAgent: (hook: PromptHook, agentId: ManagedCliAgentId, checked: boolean) => void;
  onToggleEnabled: (hook: PromptHook, enabled: boolean) => void;
}) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const queryClient = useQueryClient();
  const [tab, setTab] = useState<DetailTab>("overview");
  const [draft, setDraft] = useState<PromptHookMutationInput>(() => hookToInput(hook));
  const historyQuery = useQuery({
    enabled: hook.source === "user",
    queryKey: ["prompt-hook-history", hook.id],
    queryFn: () => service.getPromptHookVersionHistory(hook.id),
  });
  const variablesQuery = useQuery({
    enabled: hook.source === "user",
    queryKey: ["prompt-hook-variables"],
    queryFn: () => service.listPromptHookVariables(),
  });
  const history = historyQuery.data;

  useEffect(() => {
    setDraft(history?.draft?.input ?? hookToInput(hook));
  }, [history?.draft, hook]);

  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["prompt-hook-history", hook.id] }),
      queryClient.invalidateQueries({ queryKey: ["prompt-hooks"] }),
    ]);
    onChanged();
  };
  const saveMutation = useMutation({
    mutationFn: () => service.savePromptHookDraft({
      hookId: hook.id,
      expectedRevision: history?.draft?.revision ?? null,
      draft,
    }),
    onSuccess: () => void refresh(),
  });
  const publishMutation = useMutation({
    mutationFn: () => {
      if (!history?.draft) throw new Error(t("promptHooks.lifecycle.noDraft"));
      return service.publishPromptHook({
        hookId: hook.id,
        expectedDraftRevision: history.draft.revision,
        expectedPublishedVersion: history.publishedVersion ?? null,
      });
    },
    onSuccess: () => void refresh(),
  });
  const rollbackMutation = useMutation({
    mutationFn: (version: number) => service.rollbackPromptHook({
      hookId: hook.id,
      version,
      expectedPublishedVersion: history?.publishedVersion ?? null,
    }),
    onSuccess: () => void refresh(),
  });
  const resetErrors = () => {
    saveMutation.reset();
    publishMutation.reset();
    rollbackMutation.reset();
  };
  const error = saveMutation.error ?? publishMutation.error ?? rollbackMutation.error;
  const errorMessage = localizedError(error, t);

  return (
    <ApplicationDialog
      description={hook.description}
      footer={
        <DetailFooter
          draftRevision={history?.draft?.revision ?? null}
          hook={hook}
          publishDisabled={!history?.draft || publishMutation.isPending}
          savePending={saveMutation.isPending}
          tab={tab}
          onClose={onClose}
          onDelete={() => onDelete(hook)}
          onPublish={() => publishMutation.mutate()}
          onSave={() => saveMutation.mutate()}
        />
      }
      maxWidth="max-w-5xl"
      onClose={onClose}
      title={t("promptHooks.detail.title", { name: hook.name })}
    >
      {confirmationDialog}
      <DetailTabs tab={tab} userHook={hook.source === "user"} onChange={(value) => { resetErrors(); setTab(value); }} />
      <div className="mt-5">
        {tab === "overview" ? (
          <PromptHookOverview
            agents={agents}
            busy={false}
            draft={draft}
            hook={hook}
            onBindingChange={(agentId, checked) => onToggleAgent(hook, agentId, checked)}
            onDraftChange={(value) => { resetErrors(); setDraft(value); }}
            onEnabledChange={(enabled) => onToggleEnabled(hook, enabled)}
          />
        ) : null}
        {tab === "content" ? (
          <PromptHookContentEditor
            draft={draft}
            hook={hook}
            variables={variablesQuery.data ?? []}
            onChange={(value) => { resetErrors(); setDraft(value); }}
            onPreview={() => onPreview(hook)}
          />
        ) : null}
        {tab === "history" && hook.source === "user" ? (
          <PromptHookVersionHistoryView
            history={history}
            rollbackPending={rollbackMutation.isPending}
            onRollback={(version) => {
              void confirm({ title: t("promptHooks.lifecycle.rollbackConfirm", { version }) })
                .then((confirmed) => { if (confirmed) rollbackMutation.mutate(version); });
            }}
          />
        ) : null}
        {errorMessage ? <div aria-live="assertive" className="mt-4 rounded-md border px-3 py-2 text-sm ucd-status-danger">{errorMessage}</div> : null}
      </div>
    </ApplicationDialog>
  );
}

function DetailTabs({ tab, userHook, onChange }: { tab: DetailTab; userHook: boolean; onChange: (tab: DetailTab) => void }) {
  const { t } = useTranslation();
  const tabs: DetailTab[] = userHook ? ["overview", "content", "history"] : ["overview", "content"];
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const offset = event.key === "ArrowRight" ? 1 : -1;
    const next = tabs[(index + offset + tabs.length) % tabs.length];
    onChange(next);
    document.getElementById(`prompt-hook-tab-${next}`)?.focus();
  };
  return (
    <div aria-label={t("promptHooks.detail.title", { name: "" })} className="flex gap-1 overflow-x-auto border-b border-border" role="tablist">
      {tabs.map((item, index) => (
        <button
          aria-selected={tab === item}
          className={`min-h-11 whitespace-nowrap border-b-2 px-3 text-sm font-medium focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${tab === item ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"}`}
          data-dialog-autofocus={item === "overview" || undefined}
          id={`prompt-hook-tab-${item}`}
          key={item}
          onClick={() => onChange(item)}
          onKeyDown={(event) => handleKeyDown(event, index)}
          role="tab"
          tabIndex={tab === item ? 0 : -1}
          type="button"
        >
          {t(`promptHooks.detail.${item}`)}
        </button>
      ))}
    </div>
  );
}

function DetailFooter({
  draftRevision,
  hook,
  publishDisabled,
  savePending,
  tab,
  onClose,
  onDelete,
  onPublish,
  onSave,
}: {
  draftRevision: number | null;
  hook: PromptHook;
  publishDisabled: boolean;
  savePending: boolean;
  tab: DetailTab;
  onClose: () => void;
  onDelete: () => void;
  onPublish: () => void;
  onSave: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <div>{hook.source === "user" ? <Button onClick={onDelete} variant="destructive"><Trash2 aria-hidden="true" />{t("promptHooks.dialog.delete")}</Button> : null}</div>
      <div className="flex flex-wrap items-center justify-end gap-2">
        {tab === "content" && hook.source === "user" ? (
          <>
            <span className="text-xs text-muted-foreground">
              {t("promptHooks.detail.liveVersion", { version: hook.publishedVersion ?? hook.version })}
              {draftRevision == null ? "" : ` · ${t("promptHooks.lifecycle.draftRevision", { revision: draftRevision })}`}
            </span>
            <Button disabled={savePending} onClick={onSave} variant="outline">{t("promptHooks.lifecycle.saveDraft")}</Button>
            <Button disabled={publishDisabled} onClick={onPublish}>{t("promptHooks.lifecycle.publish")}</Button>
          </>
        ) : null}
        <Button onClick={onClose} variant="outline">{t("promptHooks.detail.close")}</Button>
      </div>
    </div>
  );
}

function hookToInput(hook: PromptHook): PromptHookMutationInput {
  return {
    id: hook.id,
    name: hook.name,
    description: hook.description,
    category: hook.category,
    stage: hook.stage,
    order: hook.order,
    templateBody: hook.templateBody ?? "",
    enabled: hook.enabled,
    cliBindings: [...hook.cliBindings],
    governance: { ...hook.governance },
  };
}

function localizedError(error: unknown, t: (key: string, options?: Record<string, unknown>) => string) {
  if (!error) return null;
  const message = error instanceof Error ? error.message : String(error);
  const unknown = message.match(/unsupported (?:Prompt Hook )?variables:\s*(.+)$/i)?.[1];
  return unknown
    ? t("promptHooks.lifecycle.unknownVariables", { variables: unknown })
    : t(localizePromptHookErrorKey(message));
}
