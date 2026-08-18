import { useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Cpu, LoaderCircle, Network, Play, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import type { EndpointCapabilityState, HybridDataPolicy, HybridRoutingRule, OnePieceProviderProfiles } from "../../../types/agent";

interface Props {
  overview: OnePieceProviderProfiles;
  service: AgentService;
  onSaved: (value: OnePieceProviderProfiles) => Promise<void>;
}

export function HybridLocalRuntimeSection({ overview, service, onSaved }: Props) {
  const { t } = useTranslation();
  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:11434/v1");
  const [modelId, setModelId] = useState("");
  const [runtimeKind, setRuntimeKind] = useState<"local" | "private">("local");
  const [authenticationMode, setAuthenticationMode] = useState<"none" | "optional" | "required">("none");
  const [apiKey, setApiKey] = useState("");
  const [timeoutMs, setTimeoutMs] = useState("30000");
  const [contextWindow, setContextWindow] = useState("32768");
  const [toolCapability, setToolCapability] = useState<EndpointCapabilityState>("unknown");
  const [imageCapability, setImageCapability] = useState<EndpointCapabilityState>("unknown");
  const [structuredCapability, setStructuredCapability] = useState<EndpointCapabilityState>("unknown");
  const [reasoningCapability, setReasoningCapability] = useState<EndpointCapabilityState>("unknown");
  const [policy, setPolicy] = useState<HybridDataPolicy>("local-preferred");
  const [preferredProfileId, setPreferredProfileId] = useState("");
  const [fallbackProfileId, setFallbackProfileId] = useState("");
  const [preview, setPreview] = useState<string | null>(null);
  const rulesQuery = useQuery({ queryKey: ["agents", "hybrid-routing-rules"], queryFn: () => service.listHybridRoutingRules() });
  const localProfiles = useMemo(() => overview.profiles.filter((profile) =>
    profile.sourceProviderId == null && profile.provider === "Local endpoint"), [overview.profiles]);
  const discover = useMutation({ mutationFn: () => service.discoverLocalModelEndpoints(), onSuccess: (result) => {
    const candidate = result.candidates[0];
    if (!candidate) return;
    setBaseUrl(`${candidate.baseUrl.replace(/\/$/, "")}/v1`);
    setModelId(candidate.models[0] ?? "");
    setRuntimeKind("local");
  } });
  const verify = useMutation({ mutationFn: () => service.verifyLocalModelEndpoint(baseUrl, 30_000), onSuccess: (result) => {
    const candidate = result.candidates[0];
    if (candidate?.models[0]) setModelId(candidate.models[0]);
  } });
  const saveProfile = useMutation({ mutationFn: () => service.saveCustomOnePieceProviderProfile({
    name: modelId.trim() || t("onepiece.hybrid.customProfile"),
    baseUrl,
    modelId,
    runtimeKind,
    authenticationMode,
    apiKey: apiKey.trim() || undefined,
    timeoutMs: Number(timeoutMs),
    privacyClassification: runtimeKind,
    toolCallingCapability: toolCapability,
    imageInputCapability: imageCapability,
    structuredOutputCapability: structuredCapability,
    reasoningFieldCapability: reasoningCapability,
    contextWindowTokens: contextWindow ? Number(contextWindow) : null,
    reservedOutputTokens: contextWindow ? Math.min(4096, Math.max(0, Number(contextWindow) - 1)) : 0,
  }), onSuccess: onSaved });
  const saveRule = useMutation({ mutationFn: async () => {
    const rule: HybridRoutingRule = {
      id: "hybrid-summarization",
      enabled: true,
      orderIndex: 0,
      taskClass: "summarization",
      preferredProfileId,
      fallbackProfileId: fallbackProfileId || null,
      dataPolicy: policy,
    };
    return service.replaceHybridRoutingRules([rule]);
  }, onSuccess: () => rulesQuery.refetch() });
  const previewRoute = useMutation({ mutationFn: () => service.previewHybridRoute({
    taskClass: "summarization",
    dataPolicy: policy,
    activeProfileId: overview.activeProfileId,
    hybridEnabled: true,
    requiresTools: false,
    requiresImageInput: false,
    requiresStructuredOutput: false,
    requestsReasoningField: false,
  }), onSuccess: (result) => setPreview(result.reason) });
  const error = discover.error ?? verify.error ?? saveProfile.error ?? saveRule.error ?? previewRoute.error ?? rulesQuery.error;

  return <section aria-labelledby="hybrid-local-runtime-heading" className="space-y-4 rounded-xl border border-border bg-muted/20 p-4">
    <div className="flex flex-wrap items-center gap-2">
      <Cpu className="h-4 w-4 text-primary" />
      <h3 className="font-semibold" id="hybrid-local-runtime-heading">{t("onepiece.hybrid.title")}</h3>
      <Badge tone="muted">{t("onepiece.hybrid.configuredNotVerified")}</Badge>
    </div>
    <p className="text-sm leading-6 text-muted-foreground">{t("onepiece.hybrid.description")}</p>

    <div className="grid gap-3 lg:grid-cols-2">
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.endpoint")}</span><input className="ucd-input h-10 w-full rounded-lg px-3" value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} /></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.model")}</span><input className="ucd-input h-10 w-full rounded-lg px-3" value={modelId} onChange={(event) => setModelId(event.target.value)} /></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.location")}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={runtimeKind} onChange={(event) => setRuntimeKind(event.target.value as "local" | "private")}><option value="local">{t("onepiece.hybrid.local")}</option><option value="private">{t("onepiece.hybrid.private")}</option></select></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.authentication")}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={authenticationMode} onChange={(event) => setAuthenticationMode(event.target.value as "none" | "optional" | "required")}><option value="none">{t("onepiece.hybrid.none")}</option><option value="optional">{t("onepiece.hybrid.optional")}</option><option value="required">{t("onepiece.hybrid.required")}</option></select></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.apiKey")}</span><input autoComplete="off" className="ucd-input h-10 w-full rounded-lg px-3" disabled={authenticationMode === "none"} type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} /></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.timeout")}</span><input className="ucd-input h-10 w-full rounded-lg px-3" inputMode="numeric" value={timeoutMs} onChange={(event) => setTimeoutMs(event.target.value)} /></label>
      <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.contextWindow")}</span><input className="ucd-input h-10 w-full rounded-lg px-3" inputMode="numeric" value={contextWindow} onChange={(event) => setContextWindow(event.target.value)} /></label>
      {([["tools", toolCapability, setToolCapability], ["images", imageCapability, setImageCapability], ["structured", structuredCapability, setStructuredCapability], ["reasoning", reasoningCapability, setReasoningCapability]] as const).map(([key, value, setter]) => <label className="space-y-1 text-sm" key={key}><span>{t(`onepiece.hybrid.${key}`)}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={value} onChange={(event) => setter(event.target.value as EndpointCapabilityState)}><option value="unknown">{t("onepiece.hybrid.unknown")}</option><option value="supported">{t("onepiece.hybrid.supported")}</option><option value="unsupported">{t("onepiece.hybrid.unsupported")}</option></select></label>)}
    </div>
    <div className="flex flex-wrap gap-2">
      <Button disabled={discover.isPending} onClick={() => discover.mutate()} variant="outline"><Network className="h-4 w-4" />{t("onepiece.hybrid.discover")}</Button>
      <Button disabled={verify.isPending || !baseUrl.trim()} onClick={() => verify.mutate()} variant="outline">{t("onepiece.hybrid.verify")}</Button>
      <Button disabled={saveProfile.isPending || !modelId.trim()} onClick={() => saveProfile.mutate()}><Plus className="h-4 w-4" />{t("onepiece.hybrid.saveProfile")}</Button>
    </div>

    <div className="border-t border-border pt-4">
      <h4 className="text-sm font-semibold">{t("onepiece.hybrid.routing")}</h4>
      <div className="mt-3 grid gap-3 lg:grid-cols-3">
        <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.preferred")}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={preferredProfileId} onChange={(event) => setPreferredProfileId(event.target.value)}><option value="">—</option>{localProfiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select></label>
        <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.fallback")}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={fallbackProfileId} onChange={(event) => setFallbackProfileId(event.target.value)}><option value="">—</option>{overview.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select></label>
        <label className="space-y-1 text-sm"><span>{t("onepiece.hybrid.policy")}</span><select className="ucd-input h-10 w-full rounded-lg px-3" value={policy} onChange={(event) => setPolicy(event.target.value as HybridDataPolicy)}><option value="cloud-allowed">{t("onepiece.hybrid.cloudAllowed")}</option><option value="local-preferred">{t("onepiece.hybrid.localPreferred")}</option><option value="local-only">{t("onepiece.hybrid.localOnly")}</option></select></label>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button disabled={!preferredProfileId || saveRule.isPending} onClick={() => saveRule.mutate()} size="sm">{t("onepiece.hybrid.saveRule")}</Button>
        <Button disabled={previewRoute.isPending} onClick={() => previewRoute.mutate()} size="sm" variant="outline"><Play className="h-4 w-4" />{t("onepiece.hybrid.preview")}</Button>
        {preview ? <Badge tone={preview === "waiting-local-only" ? "warning" : "muted"}>{preview}</Badge> : null}
        {(discover.isPending || verify.isPending) ? <LoaderCircle className="h-4 w-4 animate-spin" /> : null}
      </div>
    </div>
    {rulesQuery.data?.length ? <p className="text-xs text-muted-foreground">{t("onepiece.hybrid.rulesCount", { count: rulesQuery.data.length })}</p> : null}
    {error ? <p className="ucd-status-warning rounded-md border p-3 text-sm" role="alert">{error instanceof Error ? error.message : String(error)}</p> : null}
  </section>;
}
