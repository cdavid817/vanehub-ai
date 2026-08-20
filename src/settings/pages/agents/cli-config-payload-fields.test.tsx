// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../../../test/render";
import {
  payloadSupportsCredential,
  payloadSupportsEndpointOverride,
  type AntigravityConfigPayload,
  type ClaudeCodeConfigPayload,
  type GeminiCliConfigPayload,
} from "../../../types/cli-agent-config";
import { CliConfigPayloadFields } from "./cli-config-payload-fields";

const antigravity: AntigravityConfigPayload = {
  kind: "antigravity",
  toolPermission: "request-review",
  enableTerminalSandbox: false,
  verbosity: "high",
  model: "gemini-3-pro",
  advancedSettings: {},
};

const claude: ClaudeCodeConfigPayload = {
  kind: "claude-code",
  baseUrl: "https://api.example.test",
  authMode: "auth-token",
  model: "sonnet",
  haikuModel: "sonnet",
  sonnetModel: "sonnet",
  opusModel: "sonnet",
  advancedEnv: {},
};

const gemini: GeminiCliConfigPayload = {
  kind: "gemini-cli",
  baseUrl: "https://generativelanguage.googleapis.com",
  model: "auto",
  authStrategy: "api-key",
  advancedEnv: {},
};

describe("CliConfigPayloadFields", () => {
  // Antigravity authenticates through the OS keyring with Google Sign-In and accepts no
  // third-party endpoint, so rendering either control would offer the user a setting the CLI
  // cannot honor.
  it("offers no credential or endpoint field for a credential-free kind", () => {
    renderWithAppProviders(<CliConfigPayloadFields onChange={vi.fn()} payload={antigravity} />);

    expect(screen.queryByLabelText(/Base URL/i)).toBeNull();
    expect(screen.queryByLabelText(/API 密钥|API key/i)).toBeNull();
    expect(screen.queryByLabelText(/鉴权|Auth token|Authentication/i)).toBeNull();

    // The required model stays visible while optional runtime policy moves to Advanced.
    expect(screen.getByLabelText(/模型|Model/)).toBeTruthy();
    expect(screen.queryByLabelText(/工具审批|Tool approval/)).toBeNull();
  });

  it("moves optional Antigravity runtime controls to the advanced section", () => {
    renderWithAppProviders(<CliConfigPayloadFields onChange={vi.fn()} payload={antigravity} section="advanced" />);
    expect(screen.getByLabelText(/工具审批|Tool approval/)).toBeTruthy();
  });

  it("still renders the endpoint field for an endpoint-capable kind", () => {
    renderWithAppProviders(<CliConfigPayloadFields onChange={vi.fn()} payload={claude} />);

    expect(screen.getByLabelText(/Base URL/i)).toBeTruthy();
  });

  it("renders Gemini endpoint, model, and authentication controls", () => {
    renderWithAppProviders(<CliConfigPayloadFields onChange={vi.fn()} payload={gemini} />);

    expect(screen.getByLabelText(/Base URL/i)).toBeTruthy();
    expect(screen.getByLabelText(/模型|Model/)).toBeTruthy();
    expect(screen.getByLabelText(/鉴权|Authentication/i)).toBeTruthy();
    expect(payloadSupportsCredential(gemini)).toBe(true);
    expect(payloadSupportsEndpointOverride(gemini)).toBe(true);
  });

  // The dialog branches on these declarations rather than on the Agent id, so the declarations
  // themselves are what a future kind has to get right.
  it("declares the capabilities the dialog renders from", () => {
    expect(payloadSupportsCredential(antigravity)).toBe(false);
    expect(payloadSupportsEndpointOverride(antigravity)).toBe(false);
    expect(payloadSupportsCredential(claude)).toBe(true);
    expect(payloadSupportsEndpointOverride(claude)).toBe(true);
  });
});
