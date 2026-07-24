import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import type { PromptHook, PromptHookMutationInput } from "../../../types/prompt-hook";

export function PromptHookLifecyclePanel({
  hook,
  service,
  onClose,
  onChanged,
}: {
  hook: PromptHook;
  service: AgentService;
  onClose: () => void;
  onChanged: () => void;
}) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const historyQuery = useQuery({
    queryKey: ["prompt-hook-history", hook.id],
    queryFn: () => service.getPromptHookVersionHistory(hook.id),
  });
  const variablesQuery = useQuery({
    queryKey: ["prompt-hook-variables"],
    queryFn: () => service.listPromptHookVariables(),
  });
  const history = historyQuery.data;
  const [draft, setDraft] = useState<PromptHookMutationInput>(() => hookToInput(hook));

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
  const changeTemplate = (update: (current: string) => string) => {
    saveMutation.reset();
    publishMutation.reset();
    rollbackMutation.reset();
    setDraft((current) => ({ ...current, templateBody: update(current.templateBody) }));
  };
  const error = saveMutation.error ?? publishMutation.error ?? rollbackMutation.error;
  const errorMessage = error instanceof Error ? error.message : error ? String(error) : null;
  const unknownVariables = errorMessage?.match(/unsupported (?:Prompt Hook )?variables:\s*(.+)$/i)?.[1];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <section
        aria-label={t("promptHooks.lifecycle.title", { name: hook.name })}
        className="max-h-[92vh] w-full max-w-4xl overflow-auto rounded-lg bg-background p-5 shadow-xl"
      >
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-lg font-semibold">{t("promptHooks.lifecycle.title", { name: hook.name })}</h3>
            <p className="mt-1 text-sm text-muted-foreground">{t("promptHooks.lifecycle.description")}</p>
          </div>
          <Button onClick={onClose} variant="ghost">{t("promptHooks.dialog.close")}</Button>
        </div>

        <div className="mt-5 grid gap-5 lg:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
          <div className="space-y-3">
            <div>
              <div className="text-sm font-medium">{t("promptHooks.lifecycle.variables")}</div>
              <div className="mt-2 grid gap-2 sm:grid-cols-2">
              {(variablesQuery.data ?? []).map((variable) => (
                <button
                  aria-label={variable.token}
                  className="min-w-0 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-2 text-left hover:bg-accent"
                  key={variable.name}
                  onClick={() => changeTemplate((current) => (
                    `${current}${current ? " " : ""}${variable.token}`
                  ))}
                  type="button"
                >
                  <span className="block break-all font-mono text-xs font-semibold">{variable.token}</span>
                  <span className="mt-1 block text-xs text-muted-foreground">{t(variable.descriptionKey)}</span>
                  <span className="mt-1 block text-xs">
                    <span className="text-muted-foreground">{t("promptHooks.lifecycle.availability")}: </span>
                    {t(variable.availabilityKey)}
                  </span>
                  <span className="mt-1 block break-words text-xs">
                    <span className="text-muted-foreground">{t("promptHooks.lifecycle.previewExample")}: </span>
                    <span className="font-mono">{variable.example}</span>
                  </span>
                </button>
              ))}
              </div>
            </div>
            <label className="block text-sm">
              {t("promptHooks.dialog.body")}
              <textarea
                className="mt-1 min-h-56 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm"
                onChange={(event) => changeTemplate(() => event.target.value)}
                value={draft.templateBody}
              />
            </label>
            <div className="flex flex-wrap items-center gap-2">
              <Button
                disabled={saveMutation.isPending}
                onClick={() => {
                  publishMutation.reset();
                  rollbackMutation.reset();
                  saveMutation.mutate();
                }}
                variant="outline"
              >
                {t("promptHooks.lifecycle.saveDraft")}
              </Button>
              <Button
                disabled={!history?.draft || publishMutation.isPending}
                onClick={() => publishMutation.mutate()}
              >
                {t("promptHooks.lifecycle.publish")}
              </Button>
              <span className="text-xs text-muted-foreground">
                {history?.draft
                  ? t("promptHooks.lifecycle.draftRevision", { revision: history.draft.revision })
                  : t("promptHooks.lifecycle.noDraft")}
              </span>
            </div>
            {errorMessage ? (
              <div className="rounded-md border px-3 py-2 text-sm ucd-status-danger">
                {unknownVariables
                  ? t("promptHooks.lifecycle.unknownVariables", { variables: unknownVariables })
                  : errorMessage}
              </div>
            ) : null}
            <p className="text-xs text-muted-foreground">{t("promptHooks.lifecycle.variableSafety")}</p>
          </div>

          <div className="space-y-3">
            <h4 className="text-sm font-semibold">{t("promptHooks.lifecycle.history")}</h4>
            {historyQuery.isLoading ? <p className="text-sm text-muted-foreground">{t("promptHooks.loading")}</p> : null}
            {(history?.versions ?? []).map((version) => {
              const evaluation = history?.evaluations.find((item) => item.version === version.version);
              const active = history?.publishedVersion === version.version;
              return (
                <article className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3" key={version.version}>
                  <div className="flex items-center justify-between gap-2">
                    <div className="font-mono text-sm font-semibold">
                      v{version.version} {active ? `· ${t("promptHooks.lifecycle.active")}` : ""}
                    </div>
                    {!active ? (
                      <Button
                        disabled={rollbackMutation.isPending}
                        onClick={() => {
                          if (window.confirm(t("promptHooks.lifecycle.rollbackConfirm", { version: version.version }))) {
                            rollbackMutation.mutate(version.version);
                          }
                        }}
                        size="sm"
                        variant="outline"
                      >
                        {t("promptHooks.lifecycle.rollback")}
                      </Button>
                    ) : null}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" })
                      .format(new Date(version.publishedAt))}
                    {version.rollbackFromVersion
                      ? ` · ${t("promptHooks.lifecycle.rollbackFrom", { version: version.rollbackFromVersion })}`
                      : ""}
                  </div>
                  <Evaluation summary={evaluation} language={i18n.language} />
                </article>
              );
            })}
            <p className="text-xs text-muted-foreground">{t("promptHooks.lifecycle.attribution")}</p>
          </div>
        </div>
      </section>
    </div>
  );
}

function Evaluation({
  summary,
  language,
}: {
  summary: Awaited<ReturnType<AgentService["getPromptHookVersionHistory"]>>["evaluations"][number] | undefined;
  language: string;
}) {
  const { t } = useTranslation();
  if (!summary) {
    return <p className="mt-3 text-xs text-muted-foreground">{t("promptHooks.lifecycle.noEvaluation")}</p>;
  }
  const percent = new Intl.NumberFormat(language, { style: "percent", maximumFractionDigits: 1 });
  const number = new Intl.NumberFormat(language, { maximumFractionDigits: 0 });
  return (
    <dl className="mt-3 grid grid-cols-2 gap-2 text-xs">
      <Metric
        label={t("promptHooks.lifecycle.successRate")}
        value={summary.successRate == null ? "—" : percent.format(summary.successRate)}
      />
      <Metric
        label={t("promptHooks.lifecycle.averageTime")}
        value={summary.averageElapsedMs == null ? "—" : `${number.format(summary.averageElapsedMs)} ms`}
      />
      <Metric label={t("promptHooks.lifecycle.outcomes")} value={`${summary.succeededCount}/${summary.failedCount}`} />
      <Metric label={t("promptHooks.lifecycle.cancelled")} value={number.format(summary.cancelledCount)} />
    </dl>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><dt className="text-muted-foreground">{label}</dt><dd className="mt-0.5 font-mono">{value}</dd></div>;
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
