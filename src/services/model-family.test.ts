import { describe, expect, it } from "vitest";
import { normalizeModelFamily } from "./model-family";

describe("normalizeModelFamily", () => {
  // The registry stores `provider` as free-form display text, so comparing those strings directly
  // is what a cross-family check must not do.
  it("maps the built-in agents by their stable id", () => {
    expect(normalizeModelFamily({ id: "claude-code", provider: "Anthropic" })).toBe("anthropic");
    expect(normalizeModelFamily({ id: "codex-cli", provider: "OpenAI" })).toBe("openai");
    expect(normalizeModelFamily({ id: "gemini-cli", provider: "Google" })).toBe("google");
  });

  it("normalizes display-text providers regardless of casing or spacing", () => {
    expect(normalizeModelFamily({ id: "custom-1", provider: "  OpenAI " })).toBe("openai");
    expect(normalizeModelFamily({ id: "custom-2", provider: "ANTHROPIC" })).toBe("anthropic");
  });

  // OpenCode drives whichever model the user configured, so claiming a family would be a lie that
  // a cross-family reviewer check would then act on.
  it("reports opencode as unknown rather than inventing a family", () => {
    expect(normalizeModelFamily({ id: "opencode", provider: "OpenCode" })).toBe("unknown");
  });

  it("infers a family for custom API agents from their endpoint type", () => {
    expect(
      normalizeModelFamily({ id: "api-1", provider: "My Gateway", endpointType: "anthropic-messages" }),
    ).toBe("anthropic");
    expect(
      normalizeModelFamily({ id: "api-2", provider: "My Gateway", endpointType: "openai-responses" }),
    ).toBe("openai");
  });

  it("degrades an unrecognised provider to unknown instead of guessing", () => {
    expect(normalizeModelFamily({ id: "custom-3", provider: "Totally New Vendor" })).toBe("unknown");
  });

  // Two unknown families must not be treated as "the same family", or an unknown pair would be
  // wrongly rejected as same-family when recommending a reviewer.
  it("does not consider two unknown families to be the same", () => {
    expect(isSameFamily("unknown", "unknown")).toBe(false);
    expect(isSameFamily("openai", "openai")).toBe(true);
    expect(isSameFamily("openai", "anthropic")).toBe(false);
  });
});

import { isSameFamily } from "./model-family";
