import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useReducedMotion } from "../../hooks/use-reduced-motion";
import { cn } from "../../lib/utils";

export function WaitingIndicator() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();

  return (
    <span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
      <Loader2 aria-hidden="true" className={cn("h-3.5 w-3.5", !reducedMotion && "animate-spin")} />
      {t("chat.waiting")}
    </span>
  );
}
