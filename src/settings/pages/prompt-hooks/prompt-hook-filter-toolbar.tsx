import { Search } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

export function PromptHookFilterToolbar({
  categories,
  category,
  source,
  enabled,
  agent,
  agents,
  query,
  onCategoryChange,
  onSourceChange,
  onEnabledChange,
  onAgentChange,
  onQueryChange,
}: {
  categories: string[];
  category: string;
  source: string;
  enabled: string;
  agent: string;
  agents: { id: string; displayName: string }[];
  query: string;
  onCategoryChange: (category: string) => void;
  onSourceChange: (source: string) => void;
  onEnabledChange: (enabled: string) => void;
  onAgentChange: (agent: string) => void;
  onQueryChange: (query: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="ucd-panel grid gap-3 rounded-lg p-3 lg:grid-cols-[minmax(8rem,0.9fr)_minmax(8rem,0.8fr)_minmax(8rem,0.8fr)_minmax(10rem,1fr)_minmax(14rem,2fr)]">
      <Select value={category} onChange={onCategoryChange}>
        {categories.map((item) => (
          <option key={item} value={item}>
            {item === "__all__" ? t("promptHooks.filters.allCategories") : t(`promptHooks.category.${item}`)}
          </option>
        ))}
      </Select>
      <Select value={source} onChange={onSourceChange}>
        {["__all__", "builtin", "user"].map((item) => (
          <option key={item} value={item}>
            {item === "__all__" ? t("promptHooks.filters.allSources") : t(`promptHooks.source.${item}`)}
          </option>
        ))}
      </Select>
      <Select value={enabled} onChange={onEnabledChange}>
        {["__all__", "enabled", "disabled"].map((item) => (
          <option key={item} value={item}>
            {t(`promptHooks.filters.${item}`)}
          </option>
        ))}
      </Select>
      <Select value={agent} onChange={onAgentChange}>
        <option value="__all__">{t("promptHooks.filters.allAgents")}</option>
        {agents.map((item) => (
          <option key={item.id} value={item.id}>
            {item.displayName}
          </option>
        ))}
      </Select>
      <label className="relative min-w-0">
        <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
        <input
          className="h-9 w-full rounded-md border border-border bg-background py-2 pl-9 pr-3 text-sm"
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder={t("promptHooks.filters.searchPlaceholder")}
          value={query}
        />
      </label>
    </div>
  );
}

function Select({ value, children, onChange }: { value: string; children: ReactNode; onChange: (value: string) => void }) {
  return (
    <select className="h-9 rounded-md border border-border bg-background px-3 text-sm" onChange={(event) => onChange(event.target.value)} value={value}>
      {children}
    </select>
  );
}
