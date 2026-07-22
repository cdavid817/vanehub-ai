import { Bot, Users } from "lucide-react";
import { useTranslation } from "react-i18next";

export type SessionAgentMode = "single" | "multi";

export function SessionAgentModeSelector({
  mode,
  onModeChange,
}: {
  mode: SessionAgentMode;
  onModeChange: (mode: SessionAgentMode) => void;
}) {
  const { t } = useTranslation();
  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">
        {t("createSession.agentMode")}
      </span>
      <div className="grid grid-cols-2 gap-2">
        <button
          aria-pressed={mode === "single"}
          className={cnModeButton(mode === "single")}
          onClick={() => onModeChange("single")}
          type="button"
        >
          <Bot className="h-4 w-4 shrink-0" aria-hidden="true" />
          <span className="min-w-0">
            <span className="block truncate font-medium">
              {t("createSession.agentMode.single")}
            </span>
            <span className="block truncate text-xs text-muted-foreground">
              {t("createSession.agentMode.singleHint")}
            </span>
          </span>
        </button>
        <button
          aria-disabled="true"
          aria-pressed={mode === "multi"}
          className={cnModeButton(mode === "multi", true)}
          onClick={() => onModeChange("multi")}
          type="button"
        >
          <Users className="h-4 w-4 shrink-0" aria-hidden="true" />
          <span className="min-w-0">
            <span className="block truncate font-medium">
              {t("createSession.agentMode.multi")}
            </span>
            <span className="block truncate text-xs text-muted-foreground">
              {t("createSession.agentMode.comingSoon")}
            </span>
          </span>
        </button>
      </div>
    </section>
  );
}

function cnModeButton(selected: boolean, disabled = false) {
  return [
    "ucd-list-row flex min-h-12 items-center gap-2 rounded-md p-2 text-left text-sm transition",
    selected
      ? "border-primary bg-[hsl(var(--nav-active-soft))] text-foreground shadow-[0_0_0_1px_hsl(var(--primary))]"
      : "",
    disabled ? "cursor-not-allowed opacity-60" : "",
  ]
    .filter(Boolean)
    .join(" ");
}
