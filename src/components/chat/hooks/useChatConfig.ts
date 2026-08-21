import { useEffect, useMemo, useRef, useState } from "react";
import type { AgentRegistryEntry, Session } from "../../../types/agent";
import type { ChatConfig, ModelInfo, SessionExecutionMode, ReasoningDepth } from "../../../types/chat";
import { agentService } from "../../../services/runtime-agent-client";
import { EXECUTION_MODES, PROVIDER_MODELS, REASONING_DEPTHS, resolveModelLabel } from "../models";

function providerIdFromAgent(agent?: AgentRegistryEntry | null) {
  if (agent?.id === "onepiece") return "onepiece";
  const provider = agent?.provider.toLowerCase() ?? "";
  if (provider.includes("anthropic") || agent?.id.includes("claude")) return "anthropic";
  if (provider.includes("openai") || agent?.id.includes("codex")) return "openai";
  if (provider.includes("google") || agent?.id.includes("gemini")) return "google";
  if (provider.includes("opencode") || agent?.id.includes("opencode")) return "opencode";
  return "anthropic";
}

function defaultModelForProvider(providerId: string) {
  return PROVIDER_MODELS[providerId]?.[0] ?? PROVIDER_MODELS.anthropic[0];
}

function clampReasoningDepth(model: ModelInfo, depth: ReasoningDepth) {
  if (!model.supportsReasoning) return "low";
  const maxIndex = REASONING_DEPTHS.indexOf(model.maxReasoningDepth);
  const depthIndex = REASONING_DEPTHS.indexOf(depth);
  return REASONING_DEPTHS[Math.min(depthIndex, maxIndex)] ?? "low";
}

function modesForProvider() {
  return EXECUTION_MODES.map((mode) => mode.id);
}

export function useChatConfig({
  activeSession,
  agents,
  onPersistError,
  approvedPlanExit = null,
}: {
  activeSession: Session | null;
  agents: AgentRegistryEntry[];
  onPersistError?: (error: unknown) => void;
  /** Call id of an approved `exit_plan_mode` request, or null. See `plan-exit-signal`. */
  approvedPlanExit?: string | null;
}) {
  const sessionAgent = useMemo(
    () => agents.find((agent) => agent.id === activeSession?.agentId) ?? agents[0] ?? null,
    [activeSession?.agentId, agents],
  );
  const activeSessionId = activeSession?.id ?? null;
  const activeSessionAgentId = activeSession?.agentId ?? null;
  const activeInteractionMode = activeSession?.interactionMode ?? "cli";
  const initialProviderId = providerIdFromAgent(sessionAgent);
  const initialModel = defaultModelForProvider(initialProviderId);
  const [providerId, setProviderId] = useState(initialProviderId);
  const [agentId, setAgentId] = useState(sessionAgent?.id ?? "");
  const [modelId, setModelId] = useState(initialModel.id);
  const [executionMode, setSessionExecutionMode] = useState<SessionExecutionMode>("inherit");
  const [agentPolicy, setAgentPolicy] = useState<ChatConfig["agentPolicy"]>();
  const [effectiveExecutionPolicy, setEffectiveExecutionPolicy] = useState<ChatConfig["effectiveExecutionPolicy"]>();
  const [reasoningDepth, setReasoningDepth] = useState<ReasoningDepth>("high");
  const [streaming, setStreaming] = useState(true);
  const [thinking, setThinking] = useState(true);
  const [longContext, setLongContext] = useState(initialModel.supportsLongContext);
  const loadedSessionRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const nextProviderId = providerIdFromAgent(sessionAgent);
    const nextModel = defaultModelForProvider(nextProviderId);
    loadedSessionRef.current = null;
    setProviderId(nextProviderId);
    setAgentId(activeSessionAgentId ?? sessionAgent?.id ?? "");
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, "high"));
    setLongContext(nextModel.supportsLongContext);
    setSessionExecutionMode(modesForProvider()[0] ?? "inherit");
    if (!activeSessionId || !activeSessionAgentId) return () => {
      cancelled = true;
    };
    void agentService.getSessionChatConfig(activeSessionId).then((persisted) => {
      if (cancelled) return;
      setProviderId(persisted.providerId ?? nextProviderId);
      setAgentId(activeSessionAgentId);
      setModelId(persisted.modelId ?? nextModel.id);
      setSessionExecutionMode(persisted.executionMode);
      setAgentPolicy(persisted.agentPolicy);
      setEffectiveExecutionPolicy(persisted.effectiveExecutionPolicy);
      setReasoningDepth(persisted.reasoningDepth ?? "low");
      setStreaming(persisted.streaming);
      setThinking(persisted.thinking);
      setLongContext(persisted.longContext);
      loadedSessionRef.current = activeSessionId;
    }).catch(() => {
      if (!cancelled) loadedSessionRef.current = activeSessionId;
    });
    return () => {
      cancelled = true;
    };
  }, [activeSessionAgentId, activeSessionId, sessionAgent]);

  const availableAgents = useMemo(
    () => sessionAgent?.id === "onepiece"
      ? agents.filter((agent) => agent.id === "onepiece")
      : agents.filter((agent) => providerIdFromAgent(agent) === providerId),
    [agents, providerId, sessionAgent?.id],
  );
  const availableModels = useMemo(() => {
    const catalogModels = PROVIDER_MODELS[providerId] ?? [];
    if (catalogModels.some((m) => m.id === modelId)) return catalogModels;
    // Surface custom model IDs that are not in the static catalog
    const customModel: ModelInfo = {
      id: modelId,
      label: resolveModelLabel(providerId, modelId),
      providerId,
      description: "",
      supportsReasoning: false,
      maxReasoningDepth: "low",
      supportsLongContext: false,
    };
    return [customModel, ...catalogModels];
  }, [providerId, modelId]);
  const selectedModel = availableModels.find((model) => model.id === modelId) ?? availableModels[0];
  const availableModes = modesForProvider();
  const availableReasoning = REASONING_DEPTHS.filter((depth) => {
    if (!selectedModel.supportsReasoning) return false;
    return REASONING_DEPTHS.indexOf(depth) <= REASONING_DEPTHS.indexOf(selectedModel.maxReasoningDepth);
  });

  function changeProvider(nextProviderId: string) {
    const nextModel = defaultModelForProvider(nextProviderId);
    const nextAgent = agents.find((agent) => providerIdFromAgent(agent) === nextProviderId);
    const nextModes = modesForProvider();
    setProviderId(nextProviderId);
    setAgentId(nextAgent?.id ?? "");
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
    setLongContext(nextModel.supportsLongContext);
    if (!nextModes.includes(executionMode)) {
      setSessionExecutionMode(nextModes[0] ?? "inherit");
    }
  }

  function changeAgent(nextAgentId: string) {
    const nextAgent = agents.find((agent) => agent.id === nextAgentId);
    const nextProviderId = providerIdFromAgent(nextAgent);
    const nextModel = defaultModelForProvider(nextProviderId);
    const nextModes = modesForProvider();
    setAgentId(nextAgentId);
    setProviderId(nextProviderId);
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
    setLongContext(nextModel.supportsLongContext);
    if (!nextModes.includes(executionMode)) {
      setSessionExecutionMode(nextModes[0] ?? "inherit");
    }
  }

  function changeModel(nextModelId: string) {
    const nextModel = availableModels.find((model) => model.id === nextModelId)
      ?? (PROVIDER_MODELS[providerId] ?? PROVIDER_MODELS.anthropic).find((m) => m.id === nextModelId);
    setModelId(nextModelId);
    if (nextModel) {
      setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
      if (!nextModel.supportsLongContext) {
        setLongContext(false);
      }
    }
  }

  // Leaving plan mode is the model asking and the user agreeing, so the mode follows the approved
  // request rather than a mode-selector click. Keyed on the call id: re-running for the same
  // approval would fight a user who deliberately switched back to plan afterwards
  // (`add-agent-plan-exit-request`). The config effect below persists the change.
  useEffect(() => {
    if (!approvedPlanExit) return;
    setSessionExecutionMode("execute");
  }, [approvedPlanExit]);

  const config = useMemo<ChatConfig>(() => ({
    agentId,
    interactionMode: activeInteractionMode,
    executionMode,
    agentPolicy,
    effectiveExecutionPolicy,
    providerId,
    modelId,
    reasoningDepth: selectedModel.supportsReasoning ? reasoningDepth : undefined,
    streaming,
    thinking,
    longContext,
  }), [activeInteractionMode, agentId, agentPolicy, effectiveExecutionPolicy, executionMode, longContext, modelId, providerId, reasoningDepth, selectedModel.supportsReasoning, streaming, thinking]);

  useEffect(() => {
    if (!activeSessionId || loadedSessionRef.current !== activeSessionId) return;
    const timeoutId = window.setTimeout(() => {
      void agentService.saveSessionChatConfig(activeSessionId, config)
        .then((saved) => {
          setAgentPolicy(saved.agentPolicy);
          setEffectiveExecutionPolicy(saved.effectiveExecutionPolicy);
        })
        .catch((error: unknown) => {
          onPersistError?.(error);
        });
    }, 120);
    return () => window.clearTimeout(timeoutId);
  }, [activeSessionId, config, onPersistError]);

  return {
    availableAgents,
    availableModes,
    availableModels,
    availableReasoning,
    config,
    selectedModel,
    setSessionExecutionMode,
    setReasoningDepth,
    setStreaming,
    setThinking,
    setLongContext,
    changeAgent,
    changeModel,
    changeProvider,
  };
}
