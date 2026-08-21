import { ChevronDown, Ellipsis, Eye, LockKeyhole, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../../../components/measured-virtual-list";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { useMediaQuery } from "../../../hooks/use-media-query";
import { shouldVirtualizePromptHooks } from "../../../lib/virtual-list";
import type { PromptHook, PromptHookCategory } from "../../../types/prompt-hook";
import {
  flattenPromptHookGroups,
  groupPromptHooks,
  type PromptHookInventoryRow,
} from "./prompt-hook-view-model";

interface PromptHookCardListProps {
  busyHookId: string | null;
  expandedCategories: ReadonlySet<PromptHookCategory>;
  hooks: PromptHook[];
  onDelete: (hook: PromptHook) => void;
  onOpen: (hook: PromptHook) => void;
  onPreview: (hook: PromptHook) => void;
  onToggleCategory: (category: PromptHookCategory) => void;
  onToggleEnabled: (hook: PromptHook, enabled: boolean) => void;
  resetKey: string;
}

export function PromptHookCardList({
  hooks,
  busyHookId,
  expandedCategories,
  onDelete,
  onOpen,
  onPreview,
  onToggleCategory,
  onToggleEnabled,
  resetKey,
}: PromptHookCardListProps) {
  const { t } = useTranslation();
  const wideLayout = useMediaQuery("(min-width: 768px)");
  const virtualListRef = useRef<MeasuredVirtualListHandle>(null);
  const groups = useMemo(() => groupPromptHooks(hooks), [hooks]);
  const rows = useMemo(
    () => flattenPromptHookGroups(groups, expandedCategories),
    [expandedCategories, groups],
  );

  useEffect(() => {
    virtualListRef.current?.measure();
    virtualListRef.current?.scrollToStart();
  }, [resetKey, wideLayout]);

  if (hooks.length === 0) {
    return <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("promptHooks.noMatching")}</div>;
  }

  const renderRow = (row: PromptHookInventoryRow) => row.kind === "category" ? (
    <CategoryRow
      category={row.category}
      count={row.count}
      expanded={expandedCategories.has(row.category)}
      onToggle={() => onToggleCategory(row.category)}
    />
  ) : (
    <PromptHookRow
      busy={busyHookId === row.hook.id}
      hook={row.hook}
      onDelete={onDelete}
      onOpen={onOpen}
      onPreview={onPreview}
      onToggleEnabled={onToggleEnabled}
      position={row.position}
      total={row.total}
    />
  );

  if (shouldVirtualizePromptHooks(hooks.length)) {
    return (
      <MeasuredVirtualList
        ariaLabel={t("promptHooks.inventory")}
        className="h-[min(70vh,48rem)] rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2"
        estimateSize={() => 96}
        getItemKey={(row) => row.key}
        itemClassName="px-1 pb-2"
        items={rows}
        overscan={4}
        ref={virtualListRef}
        renderItem={renderRow}
        testId="prompt-hook-virtual-list"
      />
    );
  }

  return (
    <div aria-label={t("promptHooks.inventory")} className="space-y-2" role="list">
      {rows.map((row) => <div key={row.key}>{renderRow(row)}</div>)}
    </div>
  );
}

function CategoryRow({
  category,
  count,
  expanded,
  onToggle,
}: {
  category: PromptHookCategory;
  count: number;
  expanded: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const label = t(`promptHooks.category.${category}`);
  return (
    <div className="pt-2" role="presentation">
      <button
        aria-expanded={expanded}
        aria-label={t(expanded ? "promptHooks.group.collapse" : "promptHooks.group.expand", { category: label, count })}
        className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-sm font-semibold hover:bg-accent focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        onClick={onToggle}
        type="button"
      >
        <ChevronDown className={`h-4 w-4 transition-transform motion-reduce:transition-none ${expanded ? "" : "-rotate-90"}`} aria-hidden="true" />
        <span>{label}</span>
        <span className="text-xs font-normal text-muted-foreground">{count}</span>
      </button>
    </div>
  );
}

function PromptHookRow({
  busy,
  hook,
  onDelete,
  onOpen,
  onPreview,
  onToggleEnabled,
  position,
  total,
}: {
  busy: boolean;
  hook: PromptHook;
  onDelete: (hook: PromptHook) => void;
  onOpen: (hook: PromptHook) => void;
  onPreview: (hook: PromptHook) => void;
  onToggleEnabled: (hook: PromptHook, enabled: boolean) => void;
  position: number;
  total: number;
}) {
  const { t } = useTranslation();
  const version = hook.source === "user" && hook.publishedVersion == null
    ? t("promptHooks.lifecycle.unpublished")
    : `v${hook.publishedVersion ?? hook.version}`;

  return (
    <article
      aria-posinset={position}
      aria-setsize={total}
      className="ucd-panel grid gap-3 rounded-lg p-3 md:grid-cols-[minmax(0,1fr)_auto] md:items-center"
      data-hook-id={hook.id}
      role="listitem"
    >
      <button
        aria-label={t("promptHooks.card.openDetails", { name: hook.name })}
        className="min-w-0 rounded-md text-left focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => onOpen(hook)}
        type="button"
      >
        <span className="flex min-w-0 flex-wrap items-center gap-2">
          <span className="sr-only">{hook.id}</span>
          <span className="truncate font-semibold">{hook.name}</span>
          <Badge tone={hook.source === "builtin" ? "default" : "muted"}>{t(`promptHooks.source.${hook.source}`)}</Badge>
          <Badge tone={hook.enabled ? "success" : "muted"}>{hook.enabled ? t("promptHooks.enabled") : t("promptHooks.disabled")}</Badge>
          {hook.hasDraft ? <Badge tone="default">{t("promptHooks.lifecycle.draftRevision", { revision: hook.draftRevision })}</Badge> : null}
        </span>
        <span className="mt-1 flex min-w-0 flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span className="truncate">{hook.description}</span>
          <span className="font-mono">{version}</span>
          <span>{t("promptHooks.card.bindingsCount", { count: hook.cliBindings.length })}</span>
        </span>
      </button>
      <div className="flex items-center justify-between gap-2 md:justify-end">
        <label className="flex h-9 items-center gap-2 text-sm font-medium">
          {hook.disableable ? null : <LockKeyhole className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />}
          <input
            aria-label={t("promptHooks.enabled")}
            checked={hook.enabled}
            className="h-4 w-4 accent-[hsl(var(--primary))]"
            disabled={!hook.disableable || busy}
            onChange={(event) => onToggleEnabled(hook, event.target.checked)}
            type="checkbox"
          />
        </label>
        <details className="relative">
          <summary
            aria-label={t("promptHooks.actions.more", { name: hook.name })}
            className="flex h-9 w-9 cursor-pointer list-none items-center justify-center rounded-md border border-border hover:bg-accent focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          >
            <Ellipsis aria-hidden="true" />
          </summary>
          <div className="absolute right-0 z-10 mt-1 grid min-w-40 gap-1 rounded-md border border-border bg-background p-1 shadow-lg">
            <Button className="justify-start" onClick={() => onPreview(hook)} size="sm" variant="ghost">
              <Eye aria-hidden="true" />{t("promptHooks.actions.preview")}
            </Button>
            {hook.source === "user" ? (
              <Button className="justify-start" onClick={() => onDelete(hook)} size="sm" variant="ghost">
                <Trash2 aria-hidden="true" />{t("promptHooks.actions.delete")}
              </Button>
            ) : null}
          </div>
        </details>
      </div>
    </article>
  );
}
