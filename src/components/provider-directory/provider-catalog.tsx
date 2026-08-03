import { Check, Search } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ProviderBrandIcon } from "../provider-brand-icon";
import { Button } from "../ui/button";

export interface ProviderCatalogItem {
  id: string;
  displayName: string;
  category: "official" | "common";
  iconKey: string;
  catalogVersion: number;
  searchText: string;
  detail?: ReactNode;
}

export function ProviderCatalog({ items, selectedId, onSelect, title, description, searchLabel, emptyLabel }: {
  items: ProviderCatalogItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  title: string;
  description: string;
  searchLabel: string;
  emptyLabel: string;
}) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<"all" | "official" | "common">("all");
  const filtered = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    return items.filter((item) => (category === "all" || item.category === category)
      && (!query || `${item.displayName} ${item.searchText}`.toLocaleLowerCase().includes(query)));
  }, [category, items, search]);

  return (
    <section className="overflow-hidden rounded-xl border border-border bg-background">
      <div className="border-b border-border bg-muted/15 p-3 sm:p-4">
        <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(260px,0.65fr)] lg:items-end">
          <div><h3 className="font-semibold">{title}</h3><p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p></div>
          <div className="relative"><Search className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-muted-foreground" /><input aria-label={searchLabel} className="ucd-input h-10 w-full rounded-lg pl-9 pr-3 text-sm" onChange={(event) => setSearch(event.target.value)} placeholder={searchLabel} value={search} /></div>
        </div>
        <div aria-label={t("agentConfigurations.providers.categories")} className="mt-3 flex w-fit max-w-full gap-1 overflow-x-auto rounded-lg border border-border bg-background p-1" role="group">
          {(["all", "official", "common"] as const).map((value) => <Button className="h-7 shrink-0 rounded-md px-3 text-xs shadow-none" key={value} onClick={() => setCategory(value)} size="sm" variant={category === value ? "default" : "ghost"}>{t(`agentConfigurations.providers.category.${value}`)}</Button>)}
        </div>
      </div>
      <div className="p-3 sm:p-4">
        <div className="max-h-[22rem] overflow-y-auto overscroll-contain pr-1">
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {filtered.map((item) => {
            const selected = selectedId === item.id;
            return <button aria-label={item.displayName} aria-pressed={selected} className={`group relative min-h-20 rounded-xl border px-3 py-2.5 text-left transition-all ${selected ? "border-primary bg-primary/[0.06] shadow-sm ring-1 ring-primary/20" : "border-border bg-background hover:border-primary/40 hover:bg-muted/30 hover:shadow-sm"}`} key={item.id} onClick={() => onSelect(item.id)} type="button">
              {selected ? <span className="absolute right-2 top-2 flex h-5 w-5 items-center justify-center rounded-full bg-primary text-primary-foreground"><Check className="h-3 w-3" /></span> : null}
              <span className="flex items-center gap-3 pr-6">
                <ProviderBrandIcon iconKey={item.iconKey} label={item.displayName} size="sm" />
                <span className="min-w-0 flex-1"><span className="block truncate text-sm font-semibold group-hover:text-primary">{item.displayName}</span><span className="mt-0.5 block text-[11px] text-muted-foreground">{t(`agents.globalConfig.presetCategory.${item.category}`)}</span></span>
              </span>
              {item.detail ? <span className="mt-2 block border-t border-border/60 pt-2 text-xs leading-5 text-muted-foreground">{item.detail}</span> : null}
            </button>;
          })}
          </div>
          {filtered.length === 0 ? <p className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">{emptyLabel}</p> : null}
        </div>
      </div>
    </section>
  );
}
