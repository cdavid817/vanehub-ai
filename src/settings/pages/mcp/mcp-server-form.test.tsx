// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import type { McpServerConfig } from "../../../types/mcp";
import { McpServerForm } from "./mcp-server-form";

const stdioServerWithSecrets: McpServerConfig = {
  active: true,
  command: "npx",
  env: { OPENAI_API_KEY: "sk-do-not-show-this" },
  name: "has-env-secrets",
  scope: "user",
  transportType: "stdio",
};

const httpServerWithSecrets: McpServerConfig = {
  active: true,
  headers: { Authorization: "Bearer do-not-show-this-either" },
  name: "has-header-secrets",
  scope: "user",
  transportType: "streamable_http",
  url: "https://example.test/mcp",
};

const stdioServerNoSecrets: McpServerConfig = {
  active: true,
  command: "npx",
  env: {},
  name: "no-env-secrets",
  scope: "user",
  transportType: "stdio",
};

describe("McpServerForm credential masking (task 12.13)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows the env editor immediately for a brand-new server -- nothing saved to hide", () => {
    render(<McpServerForm onCancel={vi.fn()} onSave={vi.fn()} />);
    expect(screen.getByRole("textbox", { name: "Env JSON" })).toBeTruthy();
    expect(screen.queryByText(/Hidden/)).toBeNull();
  });

  it("masks an existing server's non-empty env by default, behind an explicit reveal", async () => {
    const user = userEvent.setup();
    render(<McpServerForm onCancel={vi.fn()} onSave={vi.fn()} server={stdioServerWithSecrets} />);

    expect(screen.queryByRole("textbox", { name: "Env JSON" })).toBeNull();
    expect(screen.getByText(/Hidden — may contain saved credentials/)).toBeTruthy();
    expect(screen.queryByText(/sk-do-not-show-this/)).toBeNull();
    expect(document.body.textContent).not.toContain("sk-do-not-show-this");

    await user.click(screen.getByRole("button", { name: "Reveal" }));
    const revealed = screen.getByRole("textbox", { name: "Env JSON" }) as HTMLTextAreaElement;
    expect(revealed.value).toContain("sk-do-not-show-this");
  });

  it("masks an existing server's non-empty headers the same way, for HTTP transports", async () => {
    const user = userEvent.setup();
    render(<McpServerForm onCancel={vi.fn()} onSave={vi.fn()} server={httpServerWithSecrets} />);

    expect(screen.queryByRole("textbox", { name: "Headers JSON" })).toBeNull();
    expect(document.body.textContent).not.toContain("do-not-show-this-either");

    await user.click(screen.getByRole("button", { name: "Reveal" }));
    const revealed = screen.getByRole("textbox", { name: "Headers JSON" }) as HTMLTextAreaElement;
    expect(revealed.value).toContain("do-not-show-this-either");
  });

  it("does not mask an existing server whose env is already empty", () => {
    render(<McpServerForm onCancel={vi.fn()} onSave={vi.fn()} server={stdioServerNoSecrets} />);
    expect(screen.getByRole("textbox", { name: "Env JSON" })).toBeTruthy();
    expect(screen.queryByText(/Hidden/)).toBeNull();
  });
});
