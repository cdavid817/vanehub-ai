import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Hand, Play } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentService } from "../services/agent-service";
import type { BuiltinToolOperationStatus } from "../types/builtin-tools";

export function BrowserHandoffControl({
  operationId,
  operationStatus,
  service,
}: {
  operationId: string;
  operationStatus: BuiltinToolOperationStatus;
  service: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const queryKey = ["browser", operationId, "handoff"] as const;
  const handoff = useQuery({
    enabled: operationStatus === "awaiting_human",
    queryKey,
    queryFn: () => service.getBrowserHandoff(operationId),
  });
  const begin = useMutation({
    mutationFn: () => service.beginBrowserHandoff(operationId),
    onSuccess: (state) => queryClient.setQueryData(queryKey, state),
  });
  const resume = useMutation({
    mutationFn: () => service.resumeBrowserAutomation(
      operationId,
      handoff.data?.ownershipToken ?? "",
    ),
    onSuccess: (state) => queryClient.setQueryData(queryKey, state),
  });
  const paused = handoff.data && ["awaiting_human", "human_control"].includes(handoff.data.state);

  if (paused) {
    return (
      <div className="mt-2 flex flex-wrap items-center gap-2 rounded-md border border-primary/30 bg-primary/5 p-2" role="status">
        <Hand aria-hidden="true" className="h-4 w-4 text-primary" />
        <span className="text-xs font-medium">{t("sessionTabs.browserHandoff.paused")}</span>
        <Button className="ml-auto" disabled={resume.isPending} onClick={() => resume.mutate()} size="sm">
          <Play aria-hidden="true" className="h-3.5 w-3.5" />{t("sessionTabs.browserHandoff.resume")}
        </Button>
        {resume.isError ? <span className="w-full text-xs text-destructive" role="alert">{t("sessionTabs.browserHandoff.safeError")}</span> : null}
      </div>
    );
  }
  if (handoff.data) {
    return <p className="mt-2 text-xs text-muted-foreground" role="status">{t("sessionTabs.browserHandoff.resumed")}</p>;
  }
  if (operationStatus !== "running") return null;
  return (
    <div className="mt-2">
      <Button disabled={begin.isPending} onClick={() => begin.mutate()} size="sm" variant="outline">
        <Hand aria-hidden="true" className="h-3.5 w-3.5" />{t("sessionTabs.browserHandoff.begin")}
      </Button>
      {begin.isError || handoff.isError ? <p className="mt-1 text-xs text-destructive" role="alert">{t("sessionTabs.browserHandoff.safeError")}</p> : null}
    </div>
  );
}
