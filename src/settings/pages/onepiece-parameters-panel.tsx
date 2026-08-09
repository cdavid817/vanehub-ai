import { useQuery } from "@tanstack/react-query";
import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import { OnePieceRetrievalSection } from "./agents/onepiece-retrieval-section";

export function OnePieceParametersPanel() {
  const { t } = useTranslation();
  const profiles = useQuery({
    queryKey: ["agents", "onepiece-provider-profiles"],
    queryFn: () => agentService.listOnePieceProviderProfiles(),
  });

  if (profiles.isLoading) {
    return <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  if (profiles.error) {
    return <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{profiles.error instanceof Error ? profiles.error.message : String(profiles.error)}</p>;
  }

  return <OnePieceRetrievalSection profiles={profiles.data?.profiles ?? []} />;
}
