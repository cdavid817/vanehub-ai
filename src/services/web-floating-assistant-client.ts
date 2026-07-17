import type {
  FloatingAssistantConfig,
  FloatingAssistantEvent,
  FloatingAssistantMainAction,
  FloatingAssistantSurfaceMode,
} from "../types/floating-assistant";
import type { FloatingAssistantService } from "./floating-assistant-service";

const storageKey = "vanehub.floating-assistant.v1";
const defaultConfig: FloatingAssistantConfig = { enabled: false, anchor: null };
const subscribers = new Set<(event: FloatingAssistantEvent) => void>();
let memoryConfig: FloatingAssistantConfig = defaultConfig;
let surfaceMode: FloatingAssistantSurfaceMode = "collapsed";

function normalizeConfig(value: Partial<FloatingAssistantConfig> | null): FloatingAssistantConfig {
  const anchor = value?.anchor;
  const validAnchor = anchor
    && Number.isFinite(anchor.x)
    && Number.isFinite(anchor.y)
    && Math.abs(anchor.x) <= 10_000_000
    && Math.abs(anchor.y) <= 10_000_000
    ? { x: anchor.x, y: anchor.y, monitorName: anchor.monitorName ?? null }
    : null;
  return { enabled: value?.enabled === true, anchor: validAnchor };
}

function readConfig() {
  if (typeof localStorage === "undefined") return memoryConfig;
  const raw = localStorage.getItem(storageKey);
  if (!raw) return memoryConfig;
  try {
    return normalizeConfig(JSON.parse(raw) as Partial<FloatingAssistantConfig>);
  } catch {
    return defaultConfig;
  }
}

function emit(event: FloatingAssistantEvent) {
  subscribers.forEach((handler) => handler(event));
}

function writeConfig(config: FloatingAssistantConfig) {
  memoryConfig = normalizeConfig(config);
  if (typeof localStorage !== "undefined") localStorage.setItem(storageKey, JSON.stringify(memoryConfig));
  emit({ kind: "configuration-changed", config: memoryConfig });
  return memoryConfig;
}

function emitMainAction(action: FloatingAssistantMainAction) {
  emit({ kind: "main-action", action });
}

export const webFloatingAssistantClient: FloatingAssistantService = {
  async getRuntimeInfo() {
    return { nativeAvailable: false, platform: "unsupported" };
  },
  async getConfig() {
    return readConfig();
  },
  async setEnabled(enabled) {
    return writeConfig({ ...readConfig(), enabled });
  },
  async setSurfaceMode(mode) {
    surfaceMode = mode;
    emit({ kind: "surface-changed", mode: surfaceMode });
  },
  async startDragging() {},
  async saveAnchor(anchor) {
    return writeConfig({ ...readConfig(), anchor });
  },
  async showMainWindow(action) {
    emitMainAction(action);
  },
  async exitApplication() {
    emit({ kind: "exit-requested" });
  },
  async subscribeEvents(handler) {
    subscribers.add(handler);
    return () => subscribers.delete(handler);
  },
};

export function resetWebFloatingAssistantState() {
  memoryConfig = defaultConfig;
  surfaceMode = "collapsed";
  subscribers.clear();
  if (typeof localStorage !== "undefined") localStorage.removeItem(storageKey);
}
