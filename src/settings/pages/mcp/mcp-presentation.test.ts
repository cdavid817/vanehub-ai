import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import {
  mcpConnectionStatusKey,
  mcpConnectionStatusTone,
  mcpErrorCodeFromUnknown,
  mcpErrorFromUnknown,
  mcpMutationErrorMessage,
  mcpTransportTranslationKey,
} from "./mcp-presentation";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

describe("MCP presentation safety", () => {
  it("maps every transport to a distinct user-visible translation key", () => {
    expect(mcpTransportTranslationKey("stdio")).toBe("mcp.transport.stdio");
    expect(mcpTransportTranslationKey("sse")).toBe("mcp.transport.sse");
    expect(mcpTransportTranslationKey("streamable_http")).toBe("mcp.transport.streamableHttp");
  });

  it("accepts only public safe error codes", () => {
    expect(mcpErrorCodeFromUnknown("timeout")).toBe("timeout");
    expect(mcpErrorCodeFromUnknown("future_private_code")).toBeNull();
  });

  it("does not expose an arbitrary error message without a safe code", () => {
    const unsafe = Object.assign(new Error("Authorization: secret-token"), {
      errorCode: "future_private_code",
    });
    expect(mcpErrorFromUnknown(unsafe)).toEqual({ errorCode: null, message: null });

    const safe = Object.assign(new Error("The MCP operation timed out."), { errorCode: "timeout" });
    expect(mcpErrorFromUnknown(safe)).toEqual({
      errorCode: "timeout",
      message: "The MCP operation timed out.",
    });
  });

  it("maps every connection status to a distinct label and tone, collapsing disconnected/unknown to not-tested", () => {
    expect(mcpConnectionStatusKey("connected")).toBe("mcp.status.connected");
    expect(mcpConnectionStatusKey("error")).toBe("mcp.status.error");
    expect(mcpConnectionStatusKey("disabled")).toBe("mcp.status.disabled");
    expect(mcpConnectionStatusKey("disconnected")).toBe("mcp.status.notTested");
    expect(mcpConnectionStatusKey(undefined)).toBe("mcp.status.notTested");

    expect(mcpConnectionStatusTone("connected")).toBe("success");
    expect(mcpConnectionStatusTone("error")).toBe("danger");
    expect(mcpConnectionStatusTone("disabled")).toBe("neutral");
    expect(mcpConnectionStatusTone(undefined)).toBe("neutral");
  });

  it("formats a caught mutation error through the same safe-code allowlist, never a raw message without one", () => {
    const unsafe = Object.assign(new Error("Authorization: secret-token"), { errorCode: "future_private_code" });
    expect(mcpMutationErrorMessage(t, unsafe)).toBe("The MCP operation failed. No safe error details are available.");
    expect(mcpMutationErrorMessage(t, unsafe)).not.toContain("secret-token");

    const safe = Object.assign(new Error("stdio process exited"), { errorCode: "spawn" });
    expect(mcpMutationErrorMessage(t, safe)).toBe("Process start failed [spawn]: stdio process exited");
  });
});
