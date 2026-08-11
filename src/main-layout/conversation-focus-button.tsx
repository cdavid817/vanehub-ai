import { Maximize2, Minimize2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";

export function ConversationFocusButton({ active, labelVisible = false, onToggle }: {
  active: boolean;
  labelVisible?: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const label = t(active ? "layout.focusMode.exit" : "layout.focusMode.enter");
  const Icon = active ? Minimize2 : Maximize2;

  return (
    <button
      aria-label={label}
      aria-pressed={active}
      className={cn(
        "inline-flex h-8 shrink-0 items-center justify-center gap-2 rounded-md border px-2.5 text-xs font-medium transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
        active
          ? "border-primary/50 bg-[hsl(var(--nav-active-soft))] text-primary"
          : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
      )}
      data-testid="conversation-focus-toggle"
      onClick={onToggle}
      title={label}
      type="button"
    >
      <Icon aria-hidden="true" className="h-3.5 w-3.5" />
      <span className={labelVisible ? "inline" : "hidden xl:inline"}>{label}</span>
    </button>
  );
}
