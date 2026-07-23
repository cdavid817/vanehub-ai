import { useEffect, useMemo, useRef, useState } from "react";
import type { AgentRegistryEntry, Session } from "../../../types/agent";
import type { ChatConfig, ModelInfo, PermissionMode, ReasoningDepth } from "../../../types/chat";
import { agentService } from "../../../services/runtime-agent-client";
import { PERMISSION_MODES, PROVIDER_MODELS, REASONING_DEPTHS } from "../models";

function providerIdFromAgent(agent?: AgentRegistryEntry | null) {
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

function modesForProvider(providerId: string) {
  const modeIds = PERMISSION_MODES.map((mode) => mode.id);
  if (providerId === "openai") {
    return modeIds.filter((mode) => mode !== "plan");
  }
  return modeIds;
}

export function useChatConfig({
  activeSession,
  agents,
  onPersistError,
}: {
  activeSession: Session | null;
  agents: AgentRegistryEntry[];
  onPersistError?: (error: unknown) => void;
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
  const [permissionMode, setPermissionMode] = useState<PermissionMode>("default");
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
    setPermissionMode(modesForProvider(nextProviderId)[0] ?? "default");
    if (!activeSessionId || !activeSessionAgentId) return () => {
      cancelled = true;
    };
    void agentService.getSessionChatConfig(activeSessionId).then((persisted) => {
      if (cancelled) return;
      setProviderId(persisted.providerId ?? nextProviderId);
      setAgentId(activeSessionAgentId);
      setModelId(persisted.modelId ?? nextModel.id);
      setPermissionMode(persisted.permissionMode);
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
    () => agents.filter((agent) => providerIdFromAgent(agent) === providerId),
    [agents, providerId],
  );
  const availableModels = PROVIDER_MODELS[providerId] ?? PROVIDER_MODELS.anthropic;
  const selectedModel = availableModels.find((model) => model.id === modelId) ?? availableModels[0];
  const availableModes = modesForProvider(providerId);
  const availableReasoning = REASONING_DEPTHS.filter((depth) => {
    if (!selectedModel.supportsReasoning) return false;
    return REASONING_DEPTHS.indexOf(depth) <= REASONING_DEPTHS.indexOf(selectedModel.maxReasoningDepth);
  });

  function changeProvider(nextProviderId: string) {
    const nextModel = defaultModelForProvider(nextProviderId);
    const nextAgent = agents.find((agent) => providerIdFromAgent(agent) === nextProviderId);
    const nextModes = modesForProvider(nextProviderId);
    setProviderId(nextProviderId);
    setAgentId(nextAgent?.id ?? "");
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
    setLongContext(nextModel.supportsLongContext);
    if (!nextModes.includes(permissionMode)) {
      setPermissionMode(nextModes[0] ?? "default");
    }
  }

  function changeAgent(nextAgentId: string) {
    const nextAgent = agents.find((agent) => agent.id === nextAgentId);
    const nextProviderId = providerIdFromAgent(nextAgent);
    const nextModel = defaultModelForProvider(nextProviderId);
    const nextModes = modesForProvider(nextProviderId);
    setAgentId(nextAgentId);
    setProviderId(nextProviderId);
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
    setLongContext(nextModel.supportsLongContext);
    if (!nextModes.includes(permissionMode)) {
      setPermissionMode(nextModes[0] ?? "default");
    }
  }

  function changeModel(nextModelId: string) {
    const nextModel = availableModels.find((model) => model.id === nextModelId) ?? selectedModel;
    setModelId(nextModel.id);
    setReasoningDepth(clampReasoningDepth(nextModel, reasoningDepth));
    if (!nextModel.supportsLongContext) {
      setLongContext(false);
    }
  }

  const config = useMemo<ChatConfig>(() => ({
    agentId,
    interactionMode: activeInteractionMode,
    permissionMode,
    providerId,
    modelId,
    reasoningDepth: selectedModel.supportsReasoning ? reasoningDepth : undefined,
    streaming,
    thinking,
    longContext,
  }), [activeInteractionMode, agentId, longContext, modelId, permissionMode, providerId, reasoningDepth, selectedModel.supportsReasoning, streaming, thinking]);

  useEffect(() => {
    if (!activeSessionId || loadedSessionRef.current !== activeSessionId) return;
    const timeoutId = window.setTimeout(() => {
      void agentService.saveSessionChatConfig(activeSessionId, config).catch((error: unknown) => {
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
    setPermissionMode,
    setReasoningDepth,
    setStreaming,
    setThinking,
    setLongContext,
    changeAgent,
    changeModel,
    changeProvider,
  };
}
