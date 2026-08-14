import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Ban } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentService } from "../services/agent-service";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { BuiltinToolOperation } from "../types/builtin-tools";
import { BrowserHandoffControl } from "./browser-handoff-control";

const terminalStatuses = new Set(["succeeded", "failed", "cancelled"]);

export function BuiltinToolActivity({
  service = defaultAgentService,
  sessionId,
}: {
  service?: AgentService;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const operations = useQuery({
    enabled: Boolean(sessionId),
    queryKey: ["sessions", sessionId, "builtin-tool-operations"],
    queryFn: () => service.listBuiltinToolOperations({ sessionId: sessionId ?? "", limit: 50 }),
  });
  const cancellation = useMutation({
    mutationFn: (operationId: string) => service.cancelBuiltinToolOperation(operationId),
  });

  useEffect(() => {
    if (!sessionId) return;
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void service.subscribeBuiltinToolOperations(sessionId, (event) => {
      if (disposed) return;
      queryClient.setQueryData<BuiltinToolOperation[]>(
        ["sessions", sessionId, "builtin-tool-operations"],
        (current = []) => {
        if (event.kind === "removed") return current.filter((item) => item.id !== event.operationId);
        const retained = current.filter((item) => item.id !== event.operation.id);
        return [event.operation, ...retained].slice(0, 50);
        },
      );
    }).then((stop) => {
      if (disposed) stop();
      else unsubscribe = stop;
    }).catch(() => undefined);
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [queryClient, service, sessionId]);

  if (!sessionId || (!operations.isLoading && !operations.isError && !operations.data?.length)) return null;

  return (
    <section aria-labelledby="builtin-tool-activity-heading" className="mb-4 rounded-lg border border-border bg-background p-3">
      <h3 className="text-sm font-semibold" id="builtin-tool-activity-heading">{t("sessionTabs.tools.title")}</h3>
      {operations.isLoading ? <p aria-live="polite" className="mt-2 text-xs text-muted-foreground">{t("sessionTabs.tools.loading")}</p> : null}
      {operations.isError ? <p className="mt-2 text-xs text-destructive" role="alert">{t("sessionTabs.tools.loadError")}</p> : null}
      {operations.data?.length ? (
        <ul className="mt-3 space-y-2">
          {operations.data.map((operation) => (
            <li className="rounded-md border border-border/70 bg-muted/20 p-3" key={operation.id}>
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-sm font-medium">{t(`onepiece.tools.capability.${operation.capability}`)}</span>
                <span className="font-mono text-xs text-muted-foreground">{operation.operation}</span>
                <span className="ml-auto rounded-full border border-border px-2 py-0.5 text-xs">
                  {t(`sessionTabs.tools.status.${operation.status}`)}
                </span>
                {!terminalStatuses.has(operation.status) ? (
                  <Button
                    aria-label={t("sessionTabs.tools.cancelOperation", { operation: operation.operation })}
                    disabled={cancellation.isPending}
                    onClick={() => cancellation.mutate(operation.id)}
                    size="sm"
                    variant="ghost"
                  >
                    <Ban aria-hidden="true" className="h-3.5 w-3.5" />
                    {t("sessionTabs.tools.cancel")}
                  </Button>
                ) : null}
              </div>
              {operation.progress ? (
                <div className="mt-2" role="status">
                  <div className="flex justify-between text-xs text-muted-foreground">
                    <span>{operation.progress.phase}</span>
                    <span>{progressLabel(operation)}</span>
                  </div>
                  <progress
                    aria-label={t("sessionTabs.tools.progress")}
                    className="mt-1 h-1.5 w-full accent-primary"
                    max={operation.progress.totalUnits ?? 1}
                    value={operation.progress.completedUnits ?? 0}
                  />
                </div>
              ) : null}
              {operation.errorCode ? <p className="mt-2 text-xs text-destructive">{t("sessionTabs.tools.safeFailure")}</p> : null}
              {operation.capability === "browser" ? <BrowserHandoffControl operationId={operation.id} operationStatus={operation.status} service={service} /> : null}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function progressLabel(operation: BuiltinToolOperation) {
  const progress = operation.progress;
  if (!progress || progress.completedUnits === null || progress.totalUnits === null) return "";
  return `${progress.completedUnits}/${progress.totalUnits}`;
}
