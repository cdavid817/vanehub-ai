import { Search } from "lucide-react";
import { useTranslation } from "react-i18next";

export function SkillFilterToolbar({
  categories,
  category,
  query,
  onCategoryChange,
  onQueryChange,
}: {
  categories: string[];
  category: string;
  query: string;
  onCategoryChange: (category: string) => void;
  onQueryChange: (query: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="ucd-panel flex flex-col gap-3 rounded-lg p-3 md:flex-row md:items-center">
      <select
        className="rounded-md border border-border bg-background px-3 py-2 text-sm"
        onChange={(event) => onCategoryChange(event.target.value)}
        value={category}
      >
        {categories.map((item) => (
          <option key={item} value={item}>
            {item === "__all__" ? t("skills.filters.all") : item}
          </option>
        ))}
      </select>
      <label className="relative min-w-0 flex-1">
        <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
        <input
          className="w-full rounded-md border border-border bg-background py-2 pl-9 pr-3 text-sm"
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder={t("skills.filters.searchPlaceholder")}
          value={query}
        />
      </label>
    </div>
  );
}
