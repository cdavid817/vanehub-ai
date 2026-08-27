import { describe, expect, it } from "vitest";
import {
  imPairingStartSchema,
  imSessionAccessSchema,
  imSessionBindingViewSchema,
} from "./im";

describe("IM session contracts", () => {
  it("accepts safe binding metadata and rejects external identity or delivery handles", () => {
    const safe = {
      binding: {
        connector: "telegram",
        sessionId: "session-1",
        state: "active",
        completionNotifications: false,
        createdAt: "2026-08-12T00:00:00Z",
        updatedAt: "2026-08-12T00:00:00Z",
      },
      pendingConnector: null,
      access: { connector: "feishu", enabled: false, sessionId: "session-1", updatedAt: "1970-01-01T00:00:00Z" },
    };
    expect(imSessionBindingViewSchema.safeParse(safe).success).toBe(true);
    expect(imSessionBindingViewSchema.safeParse({
      ...safe,
      binding: { ...safe.binding, externalChatId: "private-chat" },
    }).success).toBe(false);
    expect(imSessionBindingViewSchema.safeParse({
      ...safe,
      binding: { ...safe.binding, deliveryCredentialRef: "credential-ref" },
    }).success).toBe(false);
  });

  it("rejects secrets or external identities in pairing results", () => {
    const pairing = {
      connector: "telegram",
      sessionId: "session-1",
      code: "ABCD2345",
      expiresAt: "2026-08-12T00:10:00Z",
      replaceExisting: false,
    };
    expect(imPairingStartSchema.safeParse(pairing).success).toBe(true);
    expect(imPairingStartSchema.safeParse({ ...pairing, botToken: "secret" }).success).toBe(false);
    expect(imPairingStartSchema.safeParse({ ...pairing, externalChatId: "chat" }).success).toBe(false);
  });

  it("strictly validates session access without accepting native metadata drift", () => {
    const access = {
      connector: "feishu",
      enabled: false,
      sessionId: "session-1",
      updatedAt: "2026-08-12T00:00:00Z",
    };

    expect(imSessionAccessSchema.safeParse(access).success).toBe(true);
    expect(imSessionAccessSchema.safeParse({ ...access, enabled: "false" }).success).toBe(false);
    expect(imSessionAccessSchema.safeParse({ ...access, connector: "unknown" }).success).toBe(false);
    expect(imSessionAccessSchema.safeParse({ ...access, credentialRef: "secret-ref" }).success).toBe(false);
    expect(imSessionAccessSchema.safeParse({ ...access, updatedAt: undefined }).success).toBe(false);
  });
});
