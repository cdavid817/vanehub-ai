import { useQuery } from "@tanstack/react-query";
import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import { OnePieceRetrievalSection } from "./agents/onepiece-retrieval-section";
import { OnePieceContextCompactionSection } from "./agents/onepiece-context-compaction-section";
import { OnePieceContextHealthSection } from "./agents/onepiece-context-health-section";

export function OnePieceParametersPanel() {
  const { t } = useTranslation();
  // Same key *and* same shape as `OnePieceConfigurationPanel`. They used to be on different pages
  // and never mounted together, which hid a collision: one stored `{ overview, presets }` under the
  // key and the other stored the bare overview, so whichever mounted second read the wrong shape.
  // Sharing the entry also means creating a provider updates this panel immediately, because the
  // configuration panel writes the new value straight into the cache.
  const profiles = useQuery({
    queryKey: ["agents", "onepiece-provider-profiles"],
    queryFn: async () => {
      const [overview, presets] = await Promise.all([
        agentService.listOnePieceProviderProfiles(),
        agentService.listOnePieceProviderPresets(),
      ]);
      return { overview, presets };
    },
  });

  if (profiles.isLoading) {
    return <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  if (profiles.error) {
    return <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{profiles.error instanceof Error ? profiles.error.message : String(profiles.error)}</p>;
  }

  return <div className="space-y-4">
    <OnePieceContextCompactionSection />
    <OnePieceContextHealthSection />
    <OnePieceRetrievalSection profiles={profiles.data?.overview.profiles ?? []} />
  </div>;
}
