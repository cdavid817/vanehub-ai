// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { SlashCommand } from "../../services/slash-commands/types";
import { SlashCommandCompletion } from "./SlashCommandCompletion";

const command = (name: string, argumentHint?: string): SlashCommand => ({
  name, category: "runtime", argumentHint, appliesTo: () => true,
  run: async () => ({ kind: "handled" }),
});

describe("SlashCommandCompletion", () => {
  // The app defaults to zh-CN; pin English so the assertions below check real copy
  // instead of translation keys (see ModeSelect.test.tsx for the same pattern).
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing when there are no options", () => {
    const { container } = renderWithAppProviders(<SlashCommandCompletion onSelect={() => undefined} options={[]} />);
    expect(container.firstChild).toBeNull();
  });

  it("shows the invocation and the translated description", () => {
    renderWithAppProviders(
      <SlashCommandCompletion onSelect={() => undefined} options={[command("mode", "<inherit|plan|execute>")]} />,
    );
    expect(screen.getByText("/mode <inherit|plan|execute>")).not.toBeNull();
    expect(screen.getByText("Set the execution mode")).not.toBeNull();
  });

  it("reports the selected command name", async () => {
    const onSelect = vi.fn();
    const { user } = renderWithAppProviders(
      <SlashCommandCompletion onSelect={onSelect} options={[command("status"), command("usage")]} />,
    );
    await user.click(screen.getByRole("button", { name: /\/usage/ }));
    expect(onSelect).toHaveBeenCalledWith("usage");
  });
});
