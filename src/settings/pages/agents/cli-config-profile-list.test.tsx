// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import type { CliConfigProfile } from "../../../types/cli-agent-config";
import { CliConfigProfileList } from "./cli-config-profile-list";

function makeProfile(overrides: Partial<CliConfigProfile> = {}): CliConfigProfile {
  return {
    id: "profile-1",
    agentId: "claude-code",
    name: "Work profile",
    payloadVersion: 1,
    payload: { kind: "claude-code", baseUrl: "https://example.com", authMode: "api-key", model: "m", haikuModel: "m", sonnetModel: "m", opusModel: "m", advancedEnv: {} },
    sourcePresetId: null,
    sourcePresetVersion: null,
    credentialConfigured: true,
    validationState: "valid",
    appliedState: "saved",
    createdAt: "now",
    updatedAt: "now",
    ...overrides,
  };
}

describe("CliConfigProfileList actions menu keyboard navigation", () => {
  beforeAll(async () => activateAppLanguage("en"));

  function renderList() {
    render(
      <CliConfigProfileList
        busy={false}
        onApply={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onEdit={vi.fn()}
        onValidate={vi.fn()}
        presets={[]}
        profiles={[makeProfile()]}
        searchTerms={[]}
      />,
    );
  }

  it("focuses the first action when opened", () => {
    renderList();
    fireEvent.click(screen.getByRole("button", { name: "More profile actions: Work profile" }));
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Edit profile" }));
  });

  it("moves focus to the next action on ArrowDown, wrapping past the last", () => {
    renderList();
    fireEvent.click(screen.getByRole("button", { name: "More profile actions: Work profile" }));
    const edit = screen.getByRole("menuitem", { name: "Edit profile" });

    fireEvent.keyDown(edit, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Duplicate profile" }));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Delete profile" }));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(edit);
  });

  it("invokes the bound callback for the row's own profile, not a stale one", () => {
    const onEdit = vi.fn();
    render(
      <CliConfigProfileList
        busy={false}
        onApply={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onEdit={onEdit}
        onValidate={vi.fn()}
        presets={[]}
        profiles={[makeProfile({ id: "profile-2", name: "Second profile" })]}
        searchTerms={[]}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "More profile actions: Second profile" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Edit profile" }));
    expect(onEdit).toHaveBeenCalledWith(expect.objectContaining({ id: "profile-2" }));
  });
});
