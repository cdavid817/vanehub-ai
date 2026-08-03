import { describe, expect, it } from "vitest";
import {
  mcpErrorCodeFromUnknown,
  mcpErrorFromUnknown,
  mcpTransportTranslationKey,
} from "./mcp-presentation";

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
});
