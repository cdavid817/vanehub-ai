import { Bot, MessageSquarePlus } from "lucide-react";
import { useTranslation } from "react-i18next";

export function WelcomeScreen({ hasActiveSession }: { hasActiveSession: boolean }) {
  const { t } = useTranslation();

  return (
    <div className="grid h-full place-items-center p-6 text-center">
      <div className="max-w-sm">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-lg border border-border bg-background">
          {hasActiveSession ? (
            <MessageSquarePlus className="h-5 w-5 text-primary" aria-hidden="true" />
          ) : (
            <Bot className="h-5 w-5 text-primary" aria-hidden="true" />
          )}
        </div>
        <h3 className="text-sm font-semibold">{hasActiveSession ? t("chat.welcome.activeTitle") : t("chat.welcome.emptyTitle")}</h3>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          {hasActiveSession ? t("chat.welcome.activeDesc") : t("chat.welcome.emptyDesc")}
        </p>
      </div>
    </div>
  );
}
