import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { AgentRegistryEntry, RegisterApiAgentInput } from "../../../types/agent";
import { SectionPanel } from "../page-parts";

const emptyInput: RegisterApiAgentInput = {
  displayName: "",
  provider: "",
  apiKey: "",
  modelId: "",
  interfaceFormat: "openai-compatible",
  baseUrl: "",
};

const inputClass = "ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function ApiAgentRegistrationPanel({
  onCreated,
  onError,
  service = defaultAgentService,
}: {
  onCreated: (agent: AgentRegistryEntry) => void;
  onError: (message: string) => void;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [form, setForm] = useState<RegisterApiAgentInput>(emptyInput);
  const mutation = useMutation({
    mutationFn: (input: RegisterApiAgentInput) => service.registerApiAgent(input),
    onSuccess: (agent) => { setForm(emptyInput); onCreated(agent); },
    onError: (error) => onError(error instanceof Error ? error.message : String(error)),
  });

  function submit() {
    if (!form.displayName.trim() || !form.provider.trim() || !form.apiKey.trim() || !form.modelId.trim()) {
      onError(t("agents.registerApiAgent.errors.incomplete"));
      return;
    }
    if (form.interfaceFormat === "openai-compatible" && !form.baseUrl?.trim()) {
      onError(t("agents.registerApiAgent.errors.baseUrlRequired"));
      return;
    }
    void mutation.mutateAsync(form).catch(() => undefined);
  }

  return (
    <SectionPanel title={t("agents.registerApiAgent.title")} description={t("agents.registerApiAgent.description")}>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.displayName")}<input className={inputClass} onChange={(event) => setForm((value) => ({ ...value, displayName: event.target.value }))} value={form.displayName} /></label>
        <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.provider")}<input className={inputClass} onChange={(event) => setForm((value) => ({ ...value, provider: event.target.value }))} value={form.provider} /></label>
        <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.interfaceFormat")}<select className={inputClass} onChange={(event) => setForm((value) => ({ ...value, interfaceFormat: event.target.value as RegisterApiAgentInput["interfaceFormat"] }))} value={form.interfaceFormat}><option value="openai-compatible">{t("agents.registerApiAgent.interfaceFormatOpenAiCompatible")}</option><option value="anthropic">{t("agents.registerApiAgent.interfaceFormatAnthropic")}</option></select></label>
        <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.apiKey")}<input autoComplete="off" className={inputClass} onChange={(event) => setForm((value) => ({ ...value, apiKey: event.target.value }))} type="password" value={form.apiKey} /></label>
        <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.modelId")}<input className={inputClass} onChange={(event) => setForm((value) => ({ ...value, modelId: event.target.value }))} value={form.modelId} /></label>
        {form.interfaceFormat === "openai-compatible" ? <label className="flex flex-col gap-1 text-sm">{t("agents.registerApiAgent.baseUrl")}<input className={inputClass} onChange={(event) => setForm((value) => ({ ...value, baseUrl: event.target.value }))} value={form.baseUrl ?? ""} /></label> : null}
      </div>
      <Button className="mt-3" disabled={mutation.isPending} onClick={submit} variant="outline"><Plus className="h-4 w-4" />{mutation.isPending ? t("agents.registerApiAgent.submitting") : t("agents.registerApiAgent.submit")}</Button>
    </SectionPanel>
  );
}
