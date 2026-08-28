// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type {
  PersonalizationPolicy,
  PersonalizationPolicyPatch,
  PersonalizationPolicyRef,
} from "../../../types/personalization";
import { PersonalizationInstructionsView } from "./instructions-view";

const AGENT_ID = "synthetic-lab-agent";

function policy(overrides: Partial<PersonalizationPolicy> = {}): PersonalizationPolicy {
  return {
    scopeKind: "global",
    scopeKey: "",
    revision: 2,
    instructionMergeMode: "append",
    aboutUser: "Global text.",
    styleRules: "",
    memoryReadMode: "enabled",
    explicitSaveMode: "enabled",
    automaticExtractionMode: "enabled",
    globalMemoryAccessMode: "enabled",
    ...overrides,
  };
}

/**
 * A store keyed by scope, with saves that can be held open.
 *
 * Holding a save is the only way to have two of them genuinely in flight, which is the case the
 * per-scope pending state exists for.
 */
function scopedStore() {
  const stored = new Map<string, PersonalizationPolicy>([
    ["global", policy()],
    ["agent", policy({ scopeKind: "agent", scopeKey: AGENT_ID, revision: 5, aboutUser: "Agent text." })],
  ]);
  const pending: { resolve: () => void; reject: (error: Error) => void }[] = [];
  let holdNext = false;

  const patchPersonalizationPolicy = vi.fn(async (patch: PersonalizationPolicyPatch) => {
    const key = patch.scopeKind;
    const current = stored.get(key);
    const next = {
      ...(current ?? policy({ scopeKind: patch.scopeKind, scopeKey: patch.agentId ?? "" })),
      ...patch,
      revision: (current?.revision ?? 0) + 1,
    } as PersonalizationPolicy;
    if (holdNext) {
      holdNext = false;
      await new Promise<void>((resolve, reject) => {
        pending.push({ resolve: () => resolve(), reject });
      });
    }
    stored.set(key, next);
    return next;
  });

  return {
    patchPersonalizationPolicy,
    stored,
    holdNextSave: () => {
      holdNext = true;
    },
    releaseHeld: () => pending.shift()?.resolve(),
    failHeld: (message: string) => pending.shift()?.reject(new Error(message)),
    heldCount: () => pending.length,
    service: (overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) =>
      createAgentServiceDouble({
        listPersonalizationAgentCapabilities: async () => [
          {
            agentId: AGENT_ID,
            displayName: "Synthetic Lab Agent",
            supportsCustomInstructions: true,
            supportsMemoryIndex: true,
            supportsSelectedMemoryBodies: false,
            supportsAutomaticExtraction: false,
          },
        ],
        listKnownProjects: async () => [],
        listKnownRemoteWorkspaces: async () => [],
        listPersonalizationPolicies: async () => [...stored.values()],
        getPersonalizationPolicy: async (scope: PersonalizationPolicyRef) =>
          stored.get(scope.scopeKind) ?? null,
        patchPersonalizationPolicy,
        ...overrides,
      }),
  };
}

async function selectAgentScope() {
  await userEvent.selectOptions(await screen.findByTestId("personalization-scope-kind"), "agent");
  await userEvent.selectOptions(await screen.findByTestId("personalization-scope-agent"), AGENT_ID);
}

async function selectGlobalScope() {
  await userEvent.selectOptions(await screen.findByTestId("personalization-scope-kind"), "global");
}

async function typeAbout(text: string) {
  const field = await screen.findByTestId("personalization-field-aboutUser");
  await userEvent.clear(field);
  await userEvent.type(field, text);
  return field as HTMLTextAreaElement;
}

describe("instruction workflows", () => {
  it("hydrates from the store rather than from defaults", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);

    const field = (await screen.findByTestId("personalization-field-aboutUser")) as HTMLTextAreaElement;
    await waitFor(() => {
      expect(field.value).toBe("Global text.");
    });
    expect((screen.getByTestId("personalization-merge-mode") as HTMLSelectElement).value).toBe("append");
    expect(screen.getByTestId("personalization-scope-status").textContent).toContain("2");
  });

  it("hydrates an installation that has never written any layer", async () => {
    const world = scopedStore();
    world.stored.clear();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);

    const field = (await screen.findByTestId("personalization-field-aboutUser")) as HTMLTextAreaElement;
    expect(field.value).toBe("");
    expect(screen.getByTestId("personalization-scope-status").textContent).toContain("从未写入过");
    expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
  });

  it("shows each scope its own stored text", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);
    await screen.findByTestId("personalization-instruction-editor");

    await selectAgentScope();

    await waitFor(() => {
      expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
        "Agent text.",
      );
    });
  });

  it("finishes a save for the scope it was started for, not the one on screen", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);
    await screen.findByTestId("personalization-instruction-editor");

    world.holdNextSave();
    await typeAbout("Global edit.");
    await userEvent.click(screen.getByTestId("personalization-save"));
    await waitFor(() => {
      expect(world.heldCount()).toBe(1);
    });

    // Switching away mid-save is ordinary. The answer still belongs to the layer that was saved.
    await selectAgentScope();
    world.releaseHeld();

    await waitFor(() => {
      expect(world.stored.get("global")?.aboutUser).toBe("Global edit.");
    });
    // The Agent layer is untouched and still shows its own text.
    expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
      "Agent text.",
    );
    await selectGlobalScope();
    await waitFor(() => {
      expect(screen.getByTestId("personalization-scope-status").textContent).toContain("3");
    });
  });

  it("keeps one scope's failure out of another scope's draft", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);
    await screen.findByTestId("personalization-instruction-editor");

    world.holdNextSave();
    await typeAbout("Global edit.");
    await userEvent.click(screen.getByTestId("personalization-save"));
    await waitFor(() => {
      expect(world.heldCount()).toBe(1);
    });

    await selectAgentScope();
    await typeAbout("Agent edit.");
    world.failHeld("personalization-storage-unavailable");

    // The Agent draft is dirty and clean of errors; the global one is where the failure landed.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-dirty")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-save-error")).toBeNull();

    await selectGlobalScope();
    await waitFor(() => {
      expect(screen.getByTestId("personalization-save-error")).toBeTruthy();
    });
    expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
      "Global edit.",
    );
  });

  it("recovers when a retry succeeds after a failure", async () => {
    const world = scopedStore();
    let failNext = true;
    renderWithAppProviders(
      <PersonalizationInstructionsView
        service={world.service({
          patchPersonalizationPolicy: async (patch) => {
            if (failNext) {
              failNext = false;
              throw new Error("personalization-storage-unavailable");
            }
            return world.patchPersonalizationPolicy(patch);
          },
        })}
      />,
    );
    await screen.findByTestId("personalization-instruction-editor");

    await typeAbout("Worth keeping.");
    await userEvent.click(screen.getByTestId("personalization-save"));
    await screen.findByTestId("personalization-save-error");

    await userEvent.click(screen.getByTestId("personalization-save"));

    await waitFor(() => {
      expect(world.stored.get("global")?.aboutUser).toBe("Worth keeping.");
    });
    expect(screen.queryByTestId("personalization-save-error")).toBeNull();
    expect(screen.queryByTestId("personalization-dirty")).toBeNull();
  });

  it("blocks a save on either field being too long, and unblocks when it is fixed", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);

    const style = await screen.findByTestId("personalization-field-styleRules");
    await userEvent.click(style);
    await userEvent.paste("b".repeat(3001));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
    });

    await userEvent.clear(style);
    await userEvent.type(style, "Short enough.");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(false);
    });
  });

  it("follows an external change to a layer the user has not touched", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);
    await screen.findByTestId("personalization-instruction-editor");

    world.stored.set("global", policy({ revision: 8, aboutUser: "Changed elsewhere." }));
    // Leaving and returning is what a settings page does after any other panel writes; a clean
    // draft has no reason to hold a value the store no longer has.
    await selectAgentScope();
    await selectGlobalScope();

    await waitFor(() => {
      expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
        "Changed elsewhere.",
      );
    });
  });

  it("turns an external change into a conflict when the user has typed", async () => {
    const world = scopedStore();
    renderWithAppProviders(<PersonalizationInstructionsView service={world.service()} />);
    await screen.findByTestId("personalization-instruction-editor");

    await typeAbout("Mine.");
    world.stored.set("global", policy({ revision: 8, aboutUser: "Changed elsewhere." }));
    await selectAgentScope();
    await selectGlobalScope();

    // Overwriting the draft here would be the same silent loss the save path refuses.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-conflict")).toBeTruthy();
    });
    expect((screen.getByTestId("personalization-field-aboutUser") as HTMLTextAreaElement).value).toBe(
      "Mine.",
    );
  });
});
