import { imConnectorFields, type ImConnectorKind, type ImRouting } from "../../../contracts/im";

export const credentialFields = {
  feishu: imConnectorFields.feishu,
  telegram: imConnectorFields.telegram,
  dingtalk: imConnectorFields.dingtalk,
  wecom: imConnectorFields.wecom,
};

export const connectorDocumentation: Record<ImConnectorKind, string> = {
  feishu: "https://open.feishu.cn/document/server-docs/event-subscription-guide/event-subscription-configure-/request-url-configuration-case",
  telegram: "https://core.telegram.org/bots/api",
  dingtalk: "https://open.dingtalk.com/document/orgapp/stream-mode-overview",
  wecom: "https://developer.work.weixin.qq.com/document/path/101463",
  weixin: "https://ilinkai.weixin.qq.com/",
};

export function validateRouting(agentId: string, projectPath: string): { agentId?: string; projectPath?: string } {
  return {
    ...(agentId.trim() ? {} : { agentId: "required" }),
    ...(projectPath.trim() ? {} : { projectPath: "required" }),
  };
}

export function compactCredentials(values: Record<string, string>): Record<string, string> | undefined {
  const entries = Object.entries(values)
    .map(([key, value]) => [key, value.trim()] as const)
    .filter(([, value]) => value.length > 0);
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

export function hasCompleteCredentials(
  kind: Exclude<ImConnectorKind, "weixin">,
  values: Record<string, string>,
  publicConfig: Record<string, unknown> = {},
  hasStoredSecrets = false,
): boolean {
  return credentialFields[kind].filter((field) => field.required).every((field) => {
    if (field.secret && hasStoredSecrets) return true;
    const value = values[field.key] ?? (field.secret ? undefined : publicConfig[field.key]);
    return typeof value === "string" && value.trim().length > 0;
  });
}

export function credentialDraftAfterSave(
  kind: Exclude<ImConnectorKind, "weixin">,
  values: Record<string, string>,
  succeeded: boolean,
): Record<string, string> {
  if (succeeded) return {};
  const safeKeys = new Set(credentialFields[kind].filter((field) => !field.secret).map((field) => field.key));
  return Object.fromEntries(Object.entries(values).filter(([key]) => safeKeys.has(key)));
}

export function routingMatchesSaved(agentId: string, projectPath: string, savedRouting: ImRouting | null): boolean {
  return Boolean(savedRouting && savedRouting.agentId === agentId && savedRouting.projectPath === projectPath);
}
