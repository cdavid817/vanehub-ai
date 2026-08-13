import {
  imConnectorFields,
  type ImConnectorFieldDefinition,
  type ImConnectorKind,
  type WeChatAuthorization,
} from "../contracts/im";

export function mockAuthorization(
  status: WeChatAuthorization["status"],
  includeImage: boolean,
): WeChatAuthorization {
  const mockQr = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21"><rect width="21" height="21" fill="white"/><g fill="black"><path d="M1 1h7v7H1zm2 2v3h3V3zm10-2h7v7h-7zm2 2v3h3V3zM1 13h7v7H1zm2 2v3h3v-3z" fill-rule="evenodd"/><path d="M10 2h2v2h-2zm0 4h3v2h-3zm-1 3h2v3H9zm4 0h2v2h-2zm3 1h4v2h-4zm-5 3h3v2h-3zm5 0h2v3h-2zm-7 3h2v4H9zm4 1h2v3h-2zm4 1h3v2h-3z"/></g></svg>`;
  return {
    status,
    imageDataUrl: includeImage
      ? `data:image/svg+xml,${encodeURIComponent(mockQr)}`
      : null,
    expiresAt: includeImage ? new Date(Date.now() + 300_000).toISOString() : null,
    safeErrorCode: null,
  };
}

function fieldMap(kind: ImConnectorKind, secret: boolean): Map<string, ImConnectorFieldDefinition> {
  return new Map(imConnectorFields[kind].filter((field) => field.secret === secret).map((field) => [field.key, field]));
}

export function compactFieldPatch(kind: ImConnectorKind, patch?: Record<string, string>): Record<string, string> {
  const knownKeys = new Set(imConnectorFields[kind].map((field) => field.key));
  const unknownKey = Object.keys(patch ?? {}).find((key) => !knownKeys.has(key));
  if (unknownKey) throw new Error(`connector-credential-field-unknown:${unknownKey}`);
  return Object.fromEntries(
    Object.entries(patch ?? {})
      .map(([key, value]) => [key, value.trim()] as const)
      .filter(([key, value]) => knownKeys.has(key) && value.length > 0),
  );
}

export function connectorFieldMaps(kind: ImConnectorKind) {
  return { publicFields: fieldMap(kind, false), secretFields: fieldMap(kind, true) };
}
