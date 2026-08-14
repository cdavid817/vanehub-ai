// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { SlashCommandOutput } from "./SlashCommandOutput";

describe("SlashCommandOutput", () => {
  // The app defaults to zh-CN; pin English so the assertions below can check real copy
  // instead of translation keys (see ModeSelect.test.tsx for the same pattern).
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing when there is no output", () => {
    const { container } = renderWithAppProviders(<SlashCommandOutput output={null} onDismiss={() => undefined} />);
    expect(container.firstChild).toBeNull();
  });

  it("translates the title and each message", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [{ key: "slash.output.mode", params: { value: "plan" } }] }}
      />,
    );
    expect(screen.getByTestId("slash-command-output")).toBeTruthy();
    expect(screen.getByText("Applied")).toBeTruthy();
    expect(screen.getByText("Execution mode: plan")).toBeTruthy();
  });

  it("translates a help entry's description parameter before interpolating", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{
          titleKey: "slash.output.helpTitle", tone: "info",
          messages: [{ key: "slash.output.helpEntry", params: { invocation: "/status", description: "slash.command.status.description" } }],
        }}
      />,
    );
    expect(screen.getByText("/status — Show the current runtime switches")).toBeTruthy();
  });

  it("marks an error tone for assistive technology", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.error.title", tone: "error", messages: [{ key: "slash.error.notStreaming" }] }}
      />,
    );
    expect(screen.getByTestId("slash-command-output").getAttribute("data-tone")).toBe("error");
  });

  it("dismisses on the close button", async () => {
    const onDismiss = vi.fn();
    const { user } = renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={onDismiss}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [] }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Dismiss command output" }));
    expect(onDismiss).toHaveBeenCalled();
  });
});
