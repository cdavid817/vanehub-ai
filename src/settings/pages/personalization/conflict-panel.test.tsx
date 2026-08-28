// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PersonalizationPolicy } from "../../../types/personalization";
import { PersonalizationInstructionsView } from "./instructions-view";

function policy(overrides: Partial<PersonalizationPolicy> = {}): PersonalizationPolicy {
  return {
    scopeKind: "global",
    scopeKey: "",
    revision: 4,
    instructionMergeMode: "append",
    aboutUser: "Backend engineer.",
    styleRules: "Lead with the conclusion.",
    memoryReadMode: "enabled",
    explicitSaveMode: "enabled",
    automaticExtractionMode: "enabled",
    globalMemoryAccessMode: "enabled",
    ...overrides,
  };
}

/**
 * A store that moves underneath the editor.
 *
 * The first read gives revision 4; by the time the save arrives the store is at 9 with someone
 * else's text, which is exactly the race the expected-revision check exists to catch.
 */
function racingService(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  let stored = policy();
  let saves = 0;
  const patchPersonalizationPolicy = vi.fn(async (patch) => {
    saves += 1;
    if (patch.expectedRevision !== stored.revision) {
      throw new Error(
        `personalization-revision-conflict: expected ${patch.expectedRevision}, stored ${stored.revision}`,
      );
    }
    stored = { ...stored, ...patch, revision: stored.revision + 1 } as PersonalizationPolicy;
    return stored;
  });
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    listPersonalizationPolicies: async () => [stored],
    getPersonalizationPolicy: async () => stored,
    patchPersonalizationPolicy,
    ...overrides,
  });
  return {
    service,
    patchPersonalizationPolicy,
    saveCount: () => saves,
    moveStore: (next: PersonalizationPolicy) => {
      stored = next;
    },
    current: () => stored,
  };
}

async function typeAndSave(text: string) {
  const input = await screen.findByTestId("personalization-field-aboutUser");
  await userEvent.clear(input);
  await userEvent.type(input, text);
  await userEvent.click(screen.getByTestId("personalization-save"));
  return input as HTMLTextAreaElement;
}

describe("revision conflicts in the instruction editor", () => {
  it("shows both versions instead of picking one", async () => {
    const world = racingService();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    await screen.findByTestId("personalization-instruction-editor");
    world.moveStore(policy({ revision: 9, aboutUser: "Someone else's text." }));
    const input = await typeAndSave("Mine.");

    const conflict = await screen.findByTestId("personalization-conflict");
    expect(within(conflict).getByTestId("personalization-conflict-mine").textContent).toContain("Mine.");
    expect(within(conflict).getByTestId("personalization-conflict-stored").textContent).toContain(
      "Someone else's text.",
    );
    // Nothing was overwritten, and the field still holds what the user typed.
    expect(input.value).toBe("Mine.");
  });

  it("refuses to save again until the user answers", async () => {
    const world = racingService();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    await screen.findByTestId("personalization-instruction-editor");
    world.moveStore(policy({ revision: 9, aboutUser: "Theirs." }));
    await typeAndSave("Mine.");
    await screen.findByTestId("personalization-conflict");

    // Letting whichever response landed last decide is what destroys work silently, and silently
    // is the part that matters: there is no signal, so nobody thinks to check.
    expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
    expect(world.saveCount()).toBe(1);
  });

  it("lands the retry once the user chooses to keep their text", async () => {
    const world = racingService();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    await screen.findByTestId("personalization-instruction-editor");
    world.moveStore(policy({ revision: 9, aboutUser: "Theirs." }));
    await typeAndSave("Mine.");
    await screen.findByTestId("personalization-conflict");

    await userEvent.click(screen.getByTestId("personalization-conflict-keep-mine"));
    await userEvent.click(screen.getByTestId("personalization-save"));

    await waitFor(() => {
      expect(world.current().aboutUser).toBe("Mine.");
    });
    // The overwrite happened because the user chose it, not because a race decided.
    expect(world.saveCount()).toBe(2);
  });

  it("adopts the stored text and ends up with nothing to save", async () => {
    const world = racingService();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    await screen.findByTestId("personalization-instruction-editor");
    world.moveStore(policy({ revision: 9, aboutUser: "Theirs." }));
    const input = await typeAndSave("Mine.");
    await screen.findByTestId("personalization-conflict");

    await userEvent.click(screen.getByTestId("personalization-conflict-take-theirs"));

    await waitFor(() => {
      expect(input.value).toBe("Theirs.");
    });
    expect(screen.queryByTestId("personalization-conflict")).toBeNull();
    expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
  });

  it("re-reads a stored side that moved again while the conflict was open", async () => {
    const world = racingService();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    await screen.findByTestId("personalization-instruction-editor");
    world.moveStore(policy({ revision: 9, aboutUser: "First edit." }));
    await typeAndSave("Mine.");
    await screen.findByTestId("personalization-conflict");

    world.moveStore(policy({ revision: 11, aboutUser: "Second edit." }));
    await userEvent.click(screen.getByTestId("personalization-conflict-reload"));

    // Answering against a snapshot taken when the save was refused would resolve against text that
    // has moved on.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-conflict-stored").textContent).toContain("Second edit.");
    });
  });

  it("keeps the typed text through a scope change and back", async () => {
    const world = racingService({
      listPersonalizationAgentCapabilities: async () => [
        {
          agentId: "synthetic-lab-agent",
          displayName: "Synthetic Lab Agent",
          supportsCustomInstructions: true,
          supportsMemoryIndex: true,
          supportsSelectedMemoryBodies: false,
          supportsAutomaticExtraction: false,
        },
      ],
    });
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service} />);

    const input = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.clear(input);
    await userEvent.type(input, "Not saved yet.");

    await userEvent.selectOptions(screen.getByTestId("personalization-scope-kind"), "agent");
    await userEvent.selectOptions(
      await screen.findByTestId("personalization-scope-agent"),
      "synthetic-lab-agent",
    );
    await userEvent.selectOptions(screen.getByTestId("personalization-scope-kind"), "global");

    // In-app navigation needs no guard precisely because of this: drafts are keyed by scope.
    await waitFor(() => {
      expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
        "Not saved yet.",
      );
    });
  });
});
