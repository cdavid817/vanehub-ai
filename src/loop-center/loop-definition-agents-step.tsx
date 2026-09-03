import { useTranslation } from "react-i18next";
import { FormSection } from "../ui/forms/FormSection";
import type { AgentRegistryEntry } from "../types/agent";
import { Field, inputClass, type StepProps } from "./loop-definition-step-fields";

export function AgentsStep({ agents, draft, loading, setDraft }: StepProps & { agents: AgentRegistryEntry[]; loading: boolean }) {
  const { t } = useTranslation();
  // Same convention as the project and branch selects: an entry the backend would refuse stays
  // pickable (its CLI may be installed later) but says so instead of failing at save time.
  const optionLabel = (agent: AgentRegistryEntry) => agent.availabilityState === "available"
    ? agent.displayName
    : `${agent.displayName} — ${t(`createSession.agentAvailability.${agent.availabilityState}`)}`;
  return (
    <FormSection title={t("loops.editor.step.agents")}>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="loops.editor.field.worker"><select className={inputClass} disabled={loading} value={draft.workerAgentId} onChange={(event) => setDraft({ ...draft, workerAgentId: event.target.value })}><option value="">{t("loops.editor.selectAgent")}</option>{agents.map((agent) => <option key={agent.id} value={agent.id}>{optionLabel(agent)}</option>)}</select></Field>
        <Field label="loops.editor.field.verifier"><select className={inputClass} disabled={loading} value={draft.verifierAgentId} onChange={(event) => setDraft({ ...draft, verifierAgentId: event.target.value })}><option value="">{t("loops.editor.selectAgent")}</option>{agents.map((agent) => <option key={agent.id} value={agent.id}>{optionLabel(agent)}</option>)}</select></Field>
      </div>
    </FormSection>
  );
}
