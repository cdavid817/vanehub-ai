import { useQuery } from "@tanstack/react-query";
import { SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { PersonalizationPolicyRef } from "../../../types/personalization";
import { SectionPanel } from "../page-parts";
import { CustomInstructionsSection } from "./custom-instructions-section";
import { isIncomplete, PersonalizationScopeSelector } from "./scope-selector";
import { scopeKeyOf } from "./instruction-drafts";
import { useScopeOptions } from "./use-scope-options";

/**
 * The Instructions destination: which layer, then that layer's text.
 *
 * Task 10.5 replaces the editor below with one bound to the selected scope. Until then the
 * selector reports the layer's stored state, which is the same thing tasks 10.6 and 10.7 expand
 * rather than something thrown away.
 */
export function PersonalizationInstructionsView({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [scope, setScope] = useState<PersonalizationPolicyRef>({ scopeKind: "global" });
  const { agents, workspaces } = useScopeOptions(service);
  const incomplete = isIncomplete(scope);

  const policyQuery = useQuery({
    // Keyed by the scope so switching back does not show the previous layer's revision while the
    // new one loads.
    queryKey: ["personalization", "policy", scopeKeyOf(scope)] as const,
    queryFn: () => service.getPersonalizationPolicy(scope),
    enabled: !incomplete,
  });

  return (
    <div className="grid gap-5">
      <SectionPanel
        description={t("personalization.scope.description")}
        icon={SlidersHorizontal}
        title={t("personalization.scope.title")}
      >
        <PersonalizationScopeSelector
          agents={agents}
          onChange={setScope}
          scope={scope}
          workspaces={workspaces}
        />
        <p className="mt-4 text-sm text-muted-foreground" data-testid="personalization-scope-status">
          {scopeStatus()}
        </p>
      </SectionPanel>
      <CustomInstructionsSection />
    </div>
  );

  function scopeStatus(): string {
    if (incomplete) return t("personalization.scope.status.incomplete");
    if (policyQuery.isPending) return t("personalization.scope.status.loading");
    if (policyQuery.error) return t("personalization.scope.status.unavailable");
    // A layer that has never been written is not the same as one written to all-inherit: the first
    // has no revision to conflict against, and saying so is what makes the next save legible.
    if (!policyQuery.data) return t("personalization.scope.status.neverWritten");
    return t("personalization.scope.status.written", { revision: policyQuery.data.revision });
  }
}
