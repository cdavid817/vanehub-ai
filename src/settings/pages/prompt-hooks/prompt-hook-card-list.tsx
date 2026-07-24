import { Eye, History, Link2, Pencil, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../../../components/measured-virtual-list";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { useMediaQuery } from "../../../hooks/use-media-query";
import { chunkItems, shouldVirtualizePromptHooks } from "../../../lib/virtual-list";
import type { AgentRegistryEntry, ManagedCliAgentId } from "../../../types/agent";
import type { PromptHook } from "../../../types/prompt-hook";

type ManagedAgent = AgentRegistryEntry & { id: ManagedCliAgentId };

interface PromptHookCardListProps {
  agents: ManagedAgent[];
  busyHookId: string | null;
  hooks: PromptHook[];
  onDelete: (hook: PromptHook) => void;
  onEdit: (hook: PromptHook) => void;
  onAdvanced: (hook: PromptHook) => void;
  onPreview: (hook: PromptHook) => void;
  onToggleAgent: (hook: PromptHook, agentId: string, checked: boolean) => void;
  onToggleEnabled: (hook: PromptHook, enabled: boolean) => void;
  resetKey: string;
}

export function PromptHookCardList({
  hooks,
  agents,
  busyHookId,
  onToggleEnabled,
  onToggleAgent,
  onPreview,
  onEdit,
  onAdvanced,
  onDelete,
  resetKey,
}: PromptHookCardListProps) {
  const { t } = useTranslation();
  const wideLayout = useMediaQuery("(min-width: 1280px)");
  const columnCount = wideLayout ? 2 : 1;
  const virtualListRef = useRef<MeasuredVirtualListHandle>(null);
  const rows = useMemo(() => chunkItems(hooks, columnCount), [columnCount, hooks]);

  useEffect(() => {
    virtualListRef.current?.measure();
    virtualListRef.current?.scrollToStart();
  }, [columnCount, resetKey]);

  if (hooks.length === 0) {
    return <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("promptHooks.noMatching")}</div>;
  }

  const cardProps = {
    agents,
    busyHookId,
    onDelete,
    onEdit,
    onAdvanced,
    onPreview,
    onToggleAgent,
    onToggleEnabled,
    total: hooks.length,
  };

  if (shouldVirtualizePromptHooks(hooks.length)) {
    return (
      <MeasuredVirtualList
        ariaLabel={t("promptHooks.inventory")}
        className="h-[min(70vh,48rem)] rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2"
        estimateSize={() => 420}
        getItemKey={(row) => row.map((hook) => hook.id).join(":")}
        itemClassName="px-1 pb-4"
        items={rows}
        overscan={4}
        ref={virtualListRef}
        renderItem={(row, rowIndex) => (
          <div className="grid items-start gap-4 xl:grid-cols-2">
            {row.map((hook, columnIndex) => (
              <PromptHookCard
                {...cardProps}
                hook={hook}
                key={hook.id}
                position={rowIndex * columnCount + columnIndex + 1}
              />
            ))}
          </div>
        )}
        testId="prompt-hook-virtual-list"
      />
    );
  }

  return (
    <div aria-label={t("promptHooks.inventory")} className="grid items-start gap-4 xl:grid-cols-2" role="list">
      {hooks.map((hook, index) => (
        <PromptHookCard {...cardProps} hook={hook} key={hook.id} position={index + 1} />
      ))}
    </div>
  );
}

function PromptHookCard({
  agents,
  busyHookId,
  hook,
  onDelete,
  onEdit,
  onAdvanced,
  onPreview,
  onToggleAgent,
  onToggleEnabled,
  position,
  total,
}: Omit<PromptHookCardListProps, "hooks" | "resetKey"> & {
  hook: PromptHook;
  position: number;
  total: number;
}) {
  const { t } = useTranslation();

  return (
    <section
      aria-posinset={position}
      aria-setsize={total}
      className="ucd-panel grid min-h-[20rem] gap-4 rounded-lg p-4"
      role="listitem"
    >
          <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto]">
            <div className="min-w-0">
              <h3 className="truncate text-base font-semibold leading-6">{hook.name}</h3>
              <p className="mt-1 truncate font-mono text-xs text-muted-foreground">{hook.id}</p>
            </div>
            <div className="flex shrink-0 flex-wrap justify-end gap-1">
              <Badge tone={hook.source === "builtin" ? "default" : "muted"}>{t(`promptHooks.source.${hook.source}`)}</Badge>
              <Badge tone={hook.enabled ? "success" : "muted"}>{hook.enabled ? t("promptHooks.enabled") : t("promptHooks.disabled")}</Badge>
              {hook.source === "user" && hook.publishedVersion == null ? (
                <Badge tone="muted">{t("promptHooks.lifecycle.unpublished")}</Badge>
              ) : null}
              {hook.hasDraft ? (
                <Badge tone="default">
                  {t("promptHooks.lifecycle.draftRevision", { revision: hook.draftRevision })}
                </Badge>
              ) : null}
            </div>
          </div>
          <p className="line-clamp-2 min-h-10 text-sm leading-5 text-muted-foreground">{hook.description}</p>
          <div className="flex flex-wrap gap-1.5">
            <Badge tone="muted">{t(`promptHooks.category.${hook.category}`)}</Badge>
            <Badge tone="muted">{t(`promptHooks.stage.${hook.stage}`)}</Badge>
            <Badge tone="muted">{t(`promptHooks.governance.${hook.governance.governanceTier}`)}</Badge>
          </div>
          <div className="grid grid-cols-2 gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-xs text-muted-foreground md:grid-cols-4">
            <Metric label={t("promptHooks.card.order")} value={String(hook.order)} />
            <Metric
              label={t("promptHooks.card.version")}
              value={hook.publishedVersion == null && hook.source === "user"
                ? t("promptHooks.lifecycle.unpublished")
                : `v${hook.publishedVersion ?? hook.version}`}
            />
            <Metric label={t("promptHooks.card.hash")} value={hook.templateBody ? t("promptHooks.card.previewOnly") : "-"} />
            <Metric label={t("promptHooks.card.tokens")} value={t("promptHooks.card.previewOnly")} />
          </div>
          <div className="mt-auto grid gap-3 border-t border-border pt-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <label className="flex h-9 items-center gap-2 text-sm font-medium">
                <input
                  checked={hook.enabled}
                  className="h-4 w-4 accent-[hsl(var(--primary))]"
                  disabled={!hook.disableable || busyHookId === hook.id}
                  onChange={(event) => onToggleEnabled(hook, event.target.checked)}
                  type="checkbox"
                />
                {t("promptHooks.enabled")}
              </label>
              <div className="flex gap-2">
                <Button aria-label={t("promptHooks.actions.preview")} onClick={() => onPreview(hook)} size="icon" variant="outline">
                  <Eye className="h-4 w-4" aria-hidden="true" />
                </Button>
                {hook.source === "user" ? (
                  <>
                    <Button aria-label={t("promptHooks.actions.advanced")} onClick={() => onAdvanced(hook)} size="icon" variant="outline">
                      <History className="h-4 w-4" aria-hidden="true" />
                    </Button>
                    <Button aria-label={t("promptHooks.actions.edit")} onClick={() => onEdit(hook)} size="icon" variant="outline">
                      <Pencil className="h-4 w-4" aria-hidden="true" />
                    </Button>
                    <Button aria-label={t("promptHooks.actions.delete")} onClick={() => onDelete(hook)} size="icon" variant="ghost">
                      <Trash2 className="h-4 w-4" aria-hidden="true" />
                    </Button>
                  </>
                ) : null}
              </div>
            </div>
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Link2 className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              {t("promptHooks.filters.agent")}
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              {agents.map((agent) => (
                <label className="flex min-w-0 items-center gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] px-2 py-2 text-sm" key={agent.id}>
                  <input
                    className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
                    checked={hook.cliBindings.includes(agent.id)}
                    disabled={busyHookId === hook.id}
                    onChange={(event) => onToggleAgent(hook, agent.id, event.target.checked)}
                    type="checkbox"
                  />
                  <span className="min-w-0 flex-1 truncate">{agent.displayName}</span>
                </label>
              ))}
            </div>
          </div>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div>{label}</div>
      <div className="mt-1 truncate font-mono text-foreground">{value}</div>
    </div>
  );
}
