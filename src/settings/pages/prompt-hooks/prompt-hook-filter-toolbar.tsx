import { Search, SlidersHorizontal, X } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { ManagedCliAgentId } from "../../../types/agent";
import type { PromptHookCategory, PromptHookSource, PromptHookStage } from "../../../types/prompt-hook";
import {
  activeAdditionalFilterCount,
  promptHookCategoryOrder,
  type PromptHookFilters,
} from "./prompt-hook-view-model";

export function PromptHookFilterToolbar({
  agents,
  filters,
  onChange,
}: {
  agents: { id: ManagedCliAgentId; displayName: string }[];
  filters: PromptHookFilters;
  onChange: (filters: PromptHookFilters) => void;
}) {
  const { t } = useTranslation();
  const additionalCount = activeAdditionalFilterCount(filters);
  const update = <Key extends keyof PromptHookFilters>(key: Key, value: PromptHookFilters[Key]) => {
    onChange({ ...filters, [key]: value });
  };

  return (
    <div className="ucd-panel rounded-lg p-3">
      <div className="grid gap-2 xl:grid-cols-[minmax(16rem,1fr)_10rem_12rem_auto]">
        <label className="relative min-w-0">
          <span className="sr-only">{t("promptHooks.filters.searchPlaceholder")}</span>
          <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <input
            className="h-9 w-full rounded-md border border-border bg-background py-2 pl-9 pr-3 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => update("query", event.target.value)}
            placeholder={t("promptHooks.filters.searchPlaceholder")}
            value={filters.query}
          />
        </label>
        <Select
          ariaLabel={t("promptHooks.filters.__all__")}
          onChange={(value) => update("enabled", value as PromptHookFilters["enabled"])}
          value={filters.enabled}
        >
          {(["__all__", "enabled", "disabled"] as const).map((item) => (
            <option key={item} value={item}>{t(`promptHooks.filters.${item}`)}</option>
          ))}
        </Select>
        <Select
          ariaLabel={t("promptHooks.filters.agent")}
          onChange={(value) => update("agent", value as PromptHookFilters["agent"])}
          value={filters.agent}
        >
          <option value="__all__">{t("promptHooks.filters.allAgents")}</option>
          {agents.map((item) => <option key={item.id} value={item.id}>{item.displayName}</option>)}
        </Select>
        <details className="group relative">
          <summary className="flex h-9 cursor-pointer list-none items-center justify-center gap-2 rounded-md border border-border bg-[hsl(var(--panel-glass))] px-3 text-sm font-medium hover:bg-accent focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring">
            <SlidersHorizontal className="h-4 w-4" aria-hidden="true" />
            {t("promptHooks.filters.more")}
            {additionalCount > 0 ? (
              <span className="rounded-full bg-primary px-1.5 py-0.5 text-[0.65rem] text-primary-foreground">
                {t("promptHooks.filters.activeCount", { count: additionalCount })}
              </span>
            ) : null}
          </summary>
          <div className="mt-2 grid gap-2 rounded-md border border-border bg-background p-3 shadow-lg xl:absolute xl:right-0 xl:z-20 xl:w-[34rem] xl:grid-cols-3">
            <Select
              ariaLabel={t("promptHooks.filters.allSources")}
              onChange={(value) => update("source", value as PromptHookSource | "__all__")}
              value={filters.source}
            >
              {(["__all__", "builtin", "user"] as const).map((item) => (
                <option key={item} value={item}>
                  {item === "__all__" ? t("promptHooks.filters.allSources") : t(`promptHooks.source.${item}`)}
                </option>
              ))}
            </Select>
            <Select
              ariaLabel={t("promptHooks.filters.allStages")}
              onChange={(value) => update("stage", value as PromptHookStage | "__all__")}
              value={filters.stage}
            >
              <option value="__all__">{t("promptHooks.filters.allStages")}</option>
              {(["session-init", "per-turn"] as const).map((stage) => (
                <option key={stage} value={stage}>{t(`promptHooks.stage.${stage}`)}</option>
              ))}
            </Select>
            <Select
              ariaLabel={t("promptHooks.filters.allCategories")}
              onChange={(value) => update("category", value as PromptHookCategory | "__all__")}
              value={filters.category}
            >
              <option value="__all__">{t("promptHooks.filters.allCategories")}</option>
              {promptHookCategoryOrder.map((category) => (
                <option key={category} value={category}>{t(`promptHooks.category.${category}`)}</option>
              ))}
            </Select>
            {additionalCount > 0 ? (
              <Button
                className="xl:col-span-3 xl:justify-self-end"
                onClick={() => onChange({ ...filters, category: "__all__", source: "__all__", stage: "__all__" })}
                size="sm"
                variant="ghost"
              >
                <X aria-hidden="true" />
                {t("promptHooks.filters.clearAdditional")}
              </Button>
            ) : null}
          </div>
        </details>
      </div>
    </div>
  );
}

function Select({
  ariaLabel,
  children,
  value,
  onChange,
}: {
  ariaLabel: string;
  children: ReactNode;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <select
      aria-label={ariaLabel}
      className="h-9 min-w-0 rounded-md border border-border bg-background px-3 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
      onChange={(event) => onChange(event.target.value)}
      value={value}
    >
      {children}
    </select>
  );
}
