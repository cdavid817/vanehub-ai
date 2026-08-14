import { useQuery } from "@tanstack/react-query";
import { RefreshCw, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";

export function OnePieceToolReadiness({ service }: { service: AgentService }) {
  const { t } = useTranslation();
  const readiness = useQuery({
    queryKey: ["agents", "onepiece", "builtin-tool-readiness"],
    queryFn: () => service.getBuiltinToolReadiness("onepiece"),
  });

  return (
    <section aria-labelledby="onepiece-tools-heading" className="rounded-xl border border-border bg-background p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="flex items-center gap-2 font-semibold" id="onepiece-tools-heading">
            <Wrench aria-hidden="true" className="h-4 w-4 text-primary" />
            {t("onepiece.tools.title")}
          </h3>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">{t("onepiece.tools.description")}</p>
        </div>
        <Button
          aria-label={t("onepiece.tools.refresh")}
          disabled={readiness.isFetching}
          onClick={() => void readiness.refetch()}
          size="sm"
          variant="outline"
        >
          <RefreshCw aria-hidden="true" className={`h-4 w-4 ${readiness.isFetching ? "animate-spin" : ""}`} />
          <span className="hidden sm:inline">{t("onepiece.tools.refresh")}</span>
        </Button>
      </div>

      {readiness.isLoading ? <p aria-live="polite" className="mt-4 text-sm text-muted-foreground">{t("onepiece.tools.loading")}</p> : null}
      {readiness.isError ? <p className="mt-4 rounded-md border p-3 text-sm ucd-status-warning" role="alert">{t("onepiece.tools.loadError")}</p> : null}
      {readiness.data ? (
        <ul className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {readiness.data.capabilities.map((capability) => {
            const ready = capability.modes.every((mode) => mode.state === "ready");
            return (
              <li className="rounded-lg border border-border/70 bg-muted/20 p-3" key={capability.capability}>
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium">{t(`onepiece.tools.capability.${capability.capability}`)}</span>
                  <Badge tone={ready ? "success" : "warning"}>
                    {t(`onepiece.tools.state.${ready ? "ready" : "unavailable"}`)}
                  </Badge>
                </div>
                <ul aria-label={t("onepiece.tools.modes")} className="mt-2 space-y-1">
                  {capability.modes.map((mode) => (
                    <li className="flex items-center justify-between gap-2 text-xs text-muted-foreground" key={mode.mode}>
                      <span>{t(`onepiece.tools.mode.${mode.mode}`)}</span>
                      <span>{mode.reasonCode ? t(`onepiece.tools.reason.${mode.reasonCode}`) : t("onepiece.tools.state.ready")}</span>
                    </li>
                  ))}
                </ul>
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
