import type { ImConnectorKind } from "../../../contracts/im";

export interface ImCredentialField {
  key: string;
  secret: boolean;
  required?: boolean;
}

export const credentialFields: Record<Exclude<ImConnectorKind, "weixin">, ImCredentialField[]> = {
  feishu: [
    { key: "appId", secret: false, required: true },
    { key: "appSecret", secret: true, required: true },
  ],
  telegram: [{ key: "botToken", secret: true, required: true }],
  dingtalk: [
    { key: "appKey", secret: false, required: true },
    { key: "appSecret", secret: true, required: true },
    { key: "robotCode", secret: false },
  ],
  wecom: [
    { key: "botId", secret: false, required: true },
    { key: "secret", secret: true, required: true },
  ],
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

export function hasCompleteCredentials(kind: Exclude<ImConnectorKind, "weixin">, values: Record<string, string>): boolean {
  return credentialFields[kind].filter((field) => field.required).every(({ key }) => values[key]?.trim());
}
