import { z } from "zod";

export const imConnectorKindSchema = z.enum(["feishu", "telegram", "dingtalk", "wecom", "weixin"]);
export type ImConnectorKind = z.infer<typeof imConnectorKindSchema>;

export interface ImConnectorFieldDefinition {
  key: string;
  secret: boolean;
  required: boolean;
}

export const imConnectorFields: Record<ImConnectorKind, readonly ImConnectorFieldDefinition[]> = {
  feishu: [
    { key: "appId", secret: false, required: true },
    { key: "appSecret", secret: true, required: true },
  ],
  telegram: [{ key: "botToken", secret: true, required: true }],
  dingtalk: [
    { key: "appKey", secret: false, required: true },
    { key: "appSecret", secret: true, required: true },
    { key: "robotCode", secret: false, required: false },
  ],
  wecom: [
    { key: "botId", secret: false, required: true },
    { key: "secret", secret: true, required: true },
  ],
  weixin: [
    { key: "botToken", secret: true, required: true },
    { key: "baseUrl", secret: false, required: false },
    { key: "botId", secret: false, required: false },
  ],
};

export const imConnectorLifecycleSchema = z.enum([
  "unconfigured",
  "disabled",
  "connecting",
  "connected",
  "reconnecting",
  "authorization-expired",
  "error",
]);
export type ImConnectorLifecycle = z.infer<typeof imConnectorLifecycleSchema>;

export const imConnectorDescriptorSchema = z.object({
  kind: imConnectorKindSchema,
  supportsQrAuthorization: z.boolean(),
  experimental: z.boolean(),
  maxOutboundChars: z.number().int().positive(),
});
export type ImConnectorDescriptor = z.infer<typeof imConnectorDescriptorSchema>;

export const imConnectorConfigSchema = z.object({
  kind: imConnectorKindSchema,
  enabled: z.boolean(),
  displayName: z.string().nullable().optional(),
  publicConfig: z.record(z.string(), z.unknown()),
  credentialRef: z.string().nullable().optional(),
});
export type ImConnectorConfig = z.infer<typeof imConnectorConfigSchema>;

export const imConnectorHealthSchema = z.object({
  kind: imConnectorKindSchema,
  lifecycle: imConnectorLifecycleSchema,
  generation: z.number().int().nonnegative(),
  safeErrorCode: z.string().nullable().optional(),
  updatedAt: z.string(),
});
export type ImConnectorHealth = z.infer<typeof imConnectorHealthSchema>;

export const imConnectorViewSchema = z.object({
  descriptor: imConnectorDescriptorSchema,
  config: imConnectorConfigSchema,
  health: imConnectorHealthSchema,
  hasCredentials: z.boolean(),
});
export type ImConnectorView = z.infer<typeof imConnectorViewSchema>;

export const imRoutingSchema = z.object({
  agentId: z.string().min(1),
  projectPath: z.string().min(1),
});
export type ImRouting = z.infer<typeof imRoutingSchema>;

export const imBindingStateSchema = z.enum(["active", "paused"]);
export type ImBindingState = z.infer<typeof imBindingStateSchema>;

export const imSessionBindingSchema = z.object({
  connector: imConnectorKindSchema,
  sessionId: z.string().min(1),
  state: imBindingStateSchema,
  completionNotifications: z.boolean(),
  createdAt: z.string(),
  updatedAt: z.string(),
}).strict();
export type ImSessionBinding = z.infer<typeof imSessionBindingSchema>;

export const imSessionBindingViewSchema = z.object({
  binding: imSessionBindingSchema.nullable(),
  pendingConnector: imConnectorKindSchema.nullable(),
}).strict();
export type ImSessionBindingView = z.infer<typeof imSessionBindingViewSchema>;

export const imPairingStartSchema = z.object({
  connector: imConnectorKindSchema,
  sessionId: z.string().min(1),
  code: z.string().length(8),
  expiresAt: z.string(),
  replaceExisting: z.boolean(),
}).strict();
export type ImPairingStart = z.infer<typeof imPairingStartSchema>;

export interface SaveImConnectorInput {
  kind: ImConnectorKind;
  enabled: boolean;
  displayName?: string | null;
  publicConfig: Record<string, unknown>;
  credentials?: Record<string, string>;
}

export const weChatAuthorizationSchema = z.object({
  status: z.enum(["waiting", "scanned", "confirmed", "expired", "error"]),
  imageDataUrl: z.string().nullable().optional(),
  expiresAt: z.string().nullable().optional(),
  safeErrorCode: z.string().nullable().optional(),
});
export type WeChatAuthorization = z.infer<typeof weChatAuthorizationSchema>;

export function parseImConnectorViews(value: unknown): ImConnectorView[] {
  return z.array(imConnectorViewSchema).parse(value);
}

export function parseImRouting(value: unknown): ImRouting | null {
  return z.union([imRoutingSchema, z.null()]).parse(value);
}

export function parseWeChatAuthorization(value: unknown): WeChatAuthorization {
  return weChatAuthorizationSchema.parse(value);
}
