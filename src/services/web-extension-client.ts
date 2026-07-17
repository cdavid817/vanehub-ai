import type {
  ExtensionFrameworkDefinition,
  ExtensionFrameworkId,
  ExtensionFrameworkStatus,
  ExtensionOperationResult,
  ExtensionOverview,
} from "../types/extension";
import type { ExtensionService } from "./extension-service";
import { createWebMockOperation } from "./web-operation-client";

const definitions: ExtensionFrameworkDefinition[] = [
  {
    id: "paddleocr",
    capabilityId: "ocr",
    nameKey: "extensions.framework.paddleocr.name",
    descriptionKey: "extensions.framework.paddleocr.description",
    defaultPort: 9875,
    requirement: {
      runtime: "Python 3.10+",
      packages: ["paddleocr", "paddlepaddle"],
      estimatedDownloadMb: 650,
      estimatedDiskMb: 1800,
      models: [{ id: "PP-OCRv5-mobile", sizeMb: 120, descriptionKey: "extensions.model.paddleocr" }],
    },
  },
  {
    id: "faster-whisper",
    capabilityId: "asr",
    nameKey: "extensions.framework.fasterWhisper.name",
    descriptionKey: "extensions.framework.fasterWhisper.description",
    defaultPort: 9876,
    requirement: {
      runtime: "Python 3.10+",
      packages: ["faster-whisper"],
      estimatedDownloadMb: 250,
      estimatedDiskMb: 900,
      models: [{ id: "base", sizeMb: 150, descriptionKey: "extensions.model.fasterWhisper" }],
    },
  },
  {
    id: "sherpa-onnx",
    capabilityId: "tts",
    nameKey: "extensions.framework.sherpaOnnx.name",
    descriptionKey: "extensions.framework.sherpaOnnx.description",
    defaultPort: 9879,
    requirement: {
      runtime: "Python 3.10+",
      packages: ["sherpa-onnx"],
      estimatedDownloadMb: 180,
      estimatedDiskMb: 650,
      models: [{ id: "vits-zh-aishell3", sizeMb: 170, descriptionKey: "extensions.model.sherpaOnnx" }],
    },
  },
];

let statuses: ExtensionFrameworkStatus[] = definitions.map((definition) => ({
  frameworkId: definition.id,
  capabilityId: definition.capabilityId,
  status: "unsupported",
  installed: false,
  enabled: false,
  running: false,
  port: definition.defaultPort,
  installPath: null,
  installedVersion: null,
  lastHealthCheck: null,
  lastError: "extensions.environment.desktopOnly",
  lastOperationId: null,
}));

function overview(): ExtensionOverview {
  return {
    definitions,
    statuses,
    environment: {
      runtime: "web-mock",
      os: "browser",
      arch: "unknown",
      supported: false,
      nativeOperationsAvailable: false,
      pythonPath: null,
      pythonVersion: null,
      reason: "extensions.environment.desktopOnly",
    },
  };
}

function unsupportedOperation(frameworkId: ExtensionFrameworkId, action: ExtensionOperationResult["action"]) {
  const id = `web-extension-${action}-${frameworkId}-${Date.now()}`;
  statuses = statuses.map((status) =>
    status.frameworkId === frameworkId ? { ...status, lastOperationId: id } : status,
  );
  return createWebMockOperation({
    id,
    kind: "extension",
    relatedEntityId: frameworkId,
    message: `Desktop runtime required for ${action}`,
    terminalStatus: "failed",
    error: "Desktop runtime required",
    result: { success: false, frameworkId, action, message: "Desktop runtime required" },
  });
}

export const webExtensionClient: ExtensionService = {
  async getOverview() {
    return overview();
  },
  async refreshHealth() {
    return overview();
  },
  async getInstallPreview({ frameworkId }) {
    const definition = definitions.find((item) => item.id === frameworkId);
    if (!definition) throw new Error(`Unknown extension framework: ${frameworkId}`);
    return {
      frameworkId,
      supported: false,
      installPath: `~/.vanehub/extensions/${frameworkId}`,
      pythonPath: null,
      packages: definition.requirement.packages,
      models: definition.requirement.models,
      estimatedDownloadMb: definition.requirement.estimatedDownloadMb,
      estimatedDiskMb: definition.requirement.estimatedDiskMb,
      inferenceLocalOnly: true,
      reason: "extensions.environment.desktopOnly",
    };
  },
  install({ frameworkId }) {
    return Promise.resolve(unsupportedOperation(frameworkId, "install"));
  },
  uninstall({ frameworkId }) {
    return Promise.resolve(unsupportedOperation(frameworkId, "uninstall"));
  },
  setEnabled({ frameworkId, enabled }) {
    return Promise.resolve(unsupportedOperation(frameworkId, enabled ? "enable" : "disable"));
  },
  start({ frameworkId }) {
    return Promise.resolve(unsupportedOperation(frameworkId, "start"));
  },
  stop({ frameworkId }) {
    return Promise.resolve(unsupportedOperation(frameworkId, "stop"));
  },
  selfTest({ frameworkId }) {
    return Promise.resolve(unsupportedOperation(frameworkId, "self-test"));
  },
};

export function resetWebExtensionStateForTests() {
  statuses = definitions.map((definition) => ({
    frameworkId: definition.id,
    capabilityId: definition.capabilityId,
    status: "unsupported",
    installed: false,
    enabled: false,
    running: false,
    port: definition.defaultPort,
    installPath: null,
    installedVersion: null,
    lastHealthCheck: null,
    lastError: "extensions.environment.desktopOnly",
    lastOperationId: null,
  }));
}
