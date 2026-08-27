import type { ImConnectorKind, ImSessionAccess, ImSessionBinding } from "../contracts/im";
export function getSessionAccess(
  access: Map<string, ImSessionAccess>,
  sessionId: string,
  connector: ImConnectorKind,
): ImSessionAccess {
  return access.get(`${sessionId}\u0000${connector}`) ?? {
    sessionId,
    connector,
    enabled: false,
    updatedAt: "1970-01-01T00:00:00Z",
  };
}
export function mutateBinding(
  bindings: Map<string, ImSessionBinding>,
  sessionId: string,
  mutate: (binding: ImSessionBinding) => ImSessionBinding,
): ImSessionBinding {
  const binding = bindings.get(sessionId);
  if (!binding) throw new Error("im-binding-not-found");
  const next = { ...mutate(binding), updatedAt: new Date().toISOString() };
  bindings.set(sessionId, next);
  return { ...next };
}
