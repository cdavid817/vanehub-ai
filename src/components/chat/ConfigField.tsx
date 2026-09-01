import { RotateCcw, ToggleLeft, ToggleRight } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import type { ConfigFieldSource } from "./hooks/useRunConfigurationOverrides";

/**
 * One provenance-tracked row inside the Run Configuration popover (design.md Decision 9): a
 * field label, its control, and a source badge that doubles as the "this message only"
 * indicator — an override *is* a this-message-only value here, so one badge covers both
 * requirements instead of two overlapping ones. Reset only renders once there is something to
 * reset; `onReset` is omitted entirely for fields with no override path (the read-only
 * effective-policy line), which a source badge could otherwise misrepresent as adjustable.
 */
export function ConfigField({
  children,
  label,
  onReset,
  source,
}: {
  children: ReactNode;
  label: string;
  onReset?: () => void;
  source: ConfigFieldSource;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1 rounded-md border border-border/60 p-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[11px] font-medium text-muted-foreground">{label}</span>
        <div className="flex items-center gap-1">
          <Badge tone={source === "override" ? "default" : "muted"}>
            {source === "override" ? t("chat.config.sourceOverride") : t("chat.config.sourceProfile")}
          </Badge>
          {source === "override" && onReset ? (
            <button
              aria-label={t("chat.config.resetField", { label })}
              className="ucd-focus-ring rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
              onClick={onReset}
              title={t("chat.config.resetField", { label })}
              type="button"
            >
              <RotateCcw className="h-3 w-3" aria-hidden="true" />
            </button>
          ) : null}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-1.5">{children}</div>
    </div>
  );
}

/** A standalone boolean control for fields design.md groups apart from `ConfigSelect`'s own
 *  nested toggles (Reasoning/thinking's `thinking`/`streaming`, Advanced execution's
 *  `longContext`) — same visual language as those nested toggles, but wired to `setOverride`
 *  and wrapped in `ConfigField` for provenance instead of the plain on/off row `ConfigSelect`
 *  reuses as-is. */
export function ConfigToggle({
  checked,
  disabled,
  label,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onChange: (value: boolean) => void;
}) {
  const Icon = checked ? ToggleRight : ToggleLeft;
  return (
    <button
      aria-pressed={checked}
      className="flex items-center gap-1.5 rounded-md border border-border px-2 py-1.5 text-xs hover:bg-muted disabled:cursor-not-allowed disabled:opacity-60"
      disabled={disabled}
      onClick={() => onChange(!checked)}
      type="button"
    >
      <Icon className={cn("h-4 w-4", checked ? "text-primary" : "text-muted-foreground")} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}
