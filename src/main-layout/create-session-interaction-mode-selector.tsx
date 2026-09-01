import { SquareTerminal, Braces } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { InteractionMode } from "../types/agent";

/**
 * Step 1's third mode choice (task 11.3), alongside `SessionAgentModeSelector` and
 * `WorkspaceModeSelector` — mirrors `WorkspaceModeSelector`'s own disabled-button styling exactly
 * rather than inventing a second pattern for "this option has no explanation inline, the caller
 * renders one nearby when disabled" (see `create-session-step-1.tsx`).
 */
export function InteractionModeSelector({
  cliDisabled = false,
  apiDisabled = false,
  mode,
  onModeChange,
}: {
  cliDisabled?: boolean;
  apiDisabled?: boolean;
  mode: InteractionMode;
  onModeChange: (mode: InteractionMode) => void;
}) {
  const { t } = useTranslation();
  const candidates: { value: "cli" | "api"; disabled: boolean; Icon: typeof SquareTerminal }[] = [
    { value: "cli", disabled: cliDisabled, Icon: SquareTerminal },
    { value: "api", disabled: apiDisabled, Icon: Braces },
  ];
  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">{t("createSession.interactionMode")}</span>
      <div className="grid grid-cols-2 gap-2">
        {candidates.map(({ value, disabled, Icon }) => (
          <button
            className={cn(
              "ucd-list-row flex h-9 items-center justify-center gap-2 rounded-md px-3 text-xs text-foreground",
              mode === value && "ucd-choice-selected font-semibold",
              disabled && "cursor-not-allowed opacity-50",
            )}
            disabled={disabled}
            key={value}
            onClick={() => onModeChange(value)}
            type="button"
          >
            <Icon className="h-3.5 w-3.5" aria-hidden="true" />
            {t(`createSession.interactionMode.${value}`)}
          </button>
        ))}
      </div>
    </section>
  );
}
