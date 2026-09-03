import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { WorkspaceEvidenceTabId } from "../types/session-workspace-evidence";
import { consumedScopeFields, type EvidenceScopeField } from "./workspace-evidence-navigation";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";

/**
 * A chip is a promise that a filter is being applied, so only fields the destination actually
 * reads get one.
 *
 * Rendering every present field would make the bar a description of the last navigation rather
 * than of this panel: Files would show a chip for the command that led there while listing the
 * whole tree, and nothing on screen would say which of the two was true.
 */
export function WorkspaceEvidenceScopeChips({ tab }: { tab: WorkspaceEvidenceTabId }) {
  const { i18n, t } = useTranslation();
  const { clearScope, correlation, unsupportedFields } = useWorkspaceEvidenceScope();
  const chips = consumedScopeFields(tab, correlation).flatMap((field) => {
    const value = correlation[field];
    return value === undefined ? [] : [{ field, value }];
  });

  if (chips.length === 0 && unsupportedFields.length === 0) return null;

  const ignored = new Intl.ListFormat(i18n.language, { style: "long", type: "conjunction" }).format(
    unsupportedFields.map((field) => t(`workspaceScope.field.${field}`)),
  );

  return (
    <div
      aria-label={t("workspaceScope.chips.label")}
      className="flex flex-wrap items-center gap-1.5 px-1 pb-2"
      data-testid="workspace-scope-chips"
      role="group"
    >
      {chips.map(({ field, value }) => (
        <ScopeChip field={field} key={field} onClear={() => clearScope([field])} value={value} />
      ))}
      {chips.length > 0 ? (
        <button
          className="h-6 rounded border border-border px-2 text-[11px] text-muted-foreground hover:bg-muted hover:text-foreground"
          onClick={() => clearScope()}
          type="button"
        >
          {t("workspaceScope.clearAll")}
        </button>
      ) : null}
      {unsupportedFields.length > 0 ? (
        <p className="w-full text-[11px] text-muted-foreground" role="status">
          {t("workspaceScope.unsupported", { fields: ignored })}
        </p>
      ) : null}
    </div>
  );
}

function ScopeChip({
  field,
  onClear,
  value,
}: {
  field: EvidenceScopeField;
  onClear: () => void;
  value: string;
}) {
  const { t } = useTranslation();
  const label = t(`workspaceScope.field.${field}`);
  return (
    <span className="flex h-6 items-center gap-1 rounded-full border border-border bg-muted px-2 text-[11px]">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-40 truncate font-mono" title={value}>
        {value}
      </span>
      <button
        // The accessible name carries the field and the value: a row of chips otherwise presents
        // as several identically named buttons, and a screen reader user cannot tell which one
        // removes the run and which removes the trace.
        aria-label={t("workspaceScope.clearField", { field: label, value })}
        className="rounded-full p-0.5 text-muted-foreground hover:bg-background hover:text-foreground"
        onClick={onClear}
        type="button"
      >
        <X aria-hidden="true" className="h-3 w-3" />
      </button>
    </span>
  );
}
