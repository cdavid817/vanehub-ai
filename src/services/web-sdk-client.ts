import type { SdkService } from "./sdk-service";
import type {
  SdkDefinition,
  SdkId,
  SdkOperationLog,
  SdkOperationRequest,
  SdkOperationResult,
  SdkOperationType,
  SdkStatusMap,
  SdkUpdateMap,
  SdkVersionMap,
} from "../types/sdk";
import { compareSdkVersions, normalizeSdkVersion } from "./sdk-versioning";

const definitions: SdkDefinition[] = [
  {
    id: "claude-sdk",
    displayName: "Claude Code SDK",
    npmPackage: "@anthropic-ai/claude-agent-sdk",
    companionPackages: ["@anthropic-ai/sdk", "@anthropic-ai/bedrock-sdk"],
    fallbackVersions: ["0.2.88", "0.2.81", "0.2.58"],
    description: "Claude AI 功能所需，包含 Agent SDK 和 Bedrock 支持。",
    relatedProviders: ["anthropic", "bedrock"],
  },
  {
    id: "codex-sdk",
    displayName: "Codex SDK",
    npmPackage: "@openai/codex-sdk",
    companionPackages: [],
    fallbackVersions: ["0.117.0", "0.116.0", "0.115.0"],
    description: "Codex AI 功能所需。",
    relatedProviders: ["openai"],
  },
];

let statuses: SdkStatusMap = {
  "claude-sdk": {
    ...definitionStatus("claude-sdk"),
    status: "installed",
    installedVersion: "0.2.81",
    latestVersion: "0.2.88",
    hasUpdate: true,
    installPath: "~/.vanehub/dependencies/claude-sdk",
  },
  "codex-sdk": {
    ...definitionStatus("codex-sdk"),
    status: "not-installed",
    installedVersion: null,
    latestVersion: "0.117.0",
    hasUpdate: false,
    installPath: "~/.vanehub/dependencies/codex-sdk",
  },
};

const versionMap: SdkVersionMap = {
  "claude-sdk": {
    sdkId: "claude-sdk",
    versions: ["0.2.88", "0.2.81", "0.2.58"],
    fallbackVersions: ["0.2.88", "0.2.81", "0.2.58"],
    source: "remote",
    latestVersion: "0.2.88",
  },
  "codex-sdk": {
    sdkId: "codex-sdk",
    versions: ["0.117.0", "0.116.0", "0.115.0"],
    fallbackVersions: ["0.117.0", "0.116.0", "0.115.0"],
    source: "remote",
    latestVersion: "0.117.0",
  },
};

let operationLogs: SdkOperationLog[] = [];

function definitionStatus(sdkId: SdkId) {
  const definition = definitions.find((item) => item.id === sdkId);
  if (!definition) throw new Error(`Unknown SDK: ${sdkId}`);
  return {
    id: definition.id,
    displayName: definition.displayName,
    npmPackage: definition.npmPackage,
    description: definition.description,
    relatedProviders: definition.relatedProviders,
    lastChecked: new Date().toISOString(),
  };
}

function scopedVersions(sdkId?: SdkId): SdkVersionMap {
  return sdkId ? ({ [sdkId]: versionMap[sdkId] } as SdkVersionMap) : versionMap;
}

function pushLog(sdkId: SdkId, operation: SdkOperationType, line: string) {
  const log = { sdkId, operation, line, timestamp: new Date().toISOString() };
  operationLogs = [...operationLogs, log];
  return log;
}

async function simulateOperation(
  operation: SdkOperationType,
  request: SdkOperationRequest,
): Promise<SdkOperationResult> {
  const version = normalizeSdkVersion(request.version) ?? versionMap[request.sdkId].latestVersion ?? undefined;
  const logs = [
    pushLog(request.sdkId, operation, `Starting ${operation} for ${request.sdkId}`),
    pushLog(request.sdkId, operation, `Using mock npm install target ${version ?? "default"}`),
    pushLog(request.sdkId, operation, "Mock operation completed"),
  ];

  const current = statuses[request.sdkId];
  statuses = {
    ...statuses,
    [request.sdkId]: {
      ...current,
      status: "installed",
      installedVersion: version ?? current.installedVersion,
      hasUpdate: compareSdkVersions(version, current.latestVersion) < 0,
      lastChecked: new Date().toISOString(),
      errorMessage: null,
    },
  };

  return {
    success: true,
    sdkId: request.sdkId,
    operation,
    requestedVersion: version,
    installedVersion: statuses[request.sdkId].installedVersion,
    logs,
  };
}

export const webSdkClient: SdkService = {
  async listDefinitions() {
    return definitions;
  },

  async listStatuses() {
    return statuses;
  },

  async checkEnvironment() {
    return {
      available: true,
      nodePath: "web-preview-node",
      nodeVersion: "v22.0.0",
      npmPath: "web-preview-npm",
      npmVersion: "10.0.0",
    };
  },

  async getVersions(sdkId) {
    return scopedVersions(sdkId);
  },

  async checkUpdates(sdkId) {
    const ids = sdkId ? [sdkId] : definitions.map((definition) => definition.id);
    return ids.reduce<SdkUpdateMap>((updates, id) => {
      const latestVersion = versionMap[id].latestVersion ?? null;
      updates[id] = {
        id,
        latestVersion,
        hasUpdate: compareSdkVersions(statuses[id].installedVersion, latestVersion) < 0,
        errorMessage: null,
      };
      return updates;
    }, {} as SdkUpdateMap);
  },

  install(request) {
    return simulateOperation("install", request);
  },

  update(request) {
    return simulateOperation("update", request);
  },

  rollback(request) {
    return simulateOperation("rollback", request);
  },

  async uninstall(sdkId) {
    const logs = [
      pushLog(sdkId, "uninstall", `Removing ~/.vanehub/dependencies/${sdkId}`),
      pushLog(sdkId, "uninstall", "Mock uninstall completed"),
    ];
    statuses = {
      ...statuses,
      [sdkId]: {
        ...statuses[sdkId],
        status: "not-installed",
        installedVersion: null,
        hasUpdate: false,
        lastChecked: new Date().toISOString(),
      },
    };
    return { success: true, sdkId, operation: "uninstall", logs };
  },

  async getOperationLogs(sdkId) {
    return sdkId ? operationLogs.filter((log) => log.sdkId === sdkId) : operationLogs;
  },
};
