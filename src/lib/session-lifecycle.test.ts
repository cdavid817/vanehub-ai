import { describe, expect, it } from "vitest";
import en from "../i18n/locales/en.json";
import type { InteractionMode, SessionLifecycleState } from "../types/agent";
import { interactionModeLabelKey, lifecycleLabelKey, lifecycleTone } from "./session-lifecycle";

const lifecycleStates: SessionLifecycleState[] = ["idle", "starting", "running", "failed", "stopped"];
const interactionModes: InteractionMode[] = ["browser", "native-desktop", "cli", "api"];

describe("session lifecycle presentation", () => {
  it("resolves every lifecycle state to a translated key", () => {
    for (const state of lifecycleStates) {
      expect(en, state).toHaveProperty(lifecycleLabelKey(state));
    }
  });

  it("resolves every interaction mode to a translated key", () => {
    for (const mode of interactionModes) {
      expect(en, mode).toHaveProperty(interactionModeLabelKey(mode));
    }
  });

  /**
   * The bug this guards: `failed` used to reach the session list through a separate label map
   * that rendered it as "needs input" while the info panel called the same state "failed".
   */
  it("gives failure its own tone instead of sharing the healthy one", () => {
    expect(lifecycleTone("failed")).toBe("danger");
    expect(lifecycleTone("running")).toBe("active");
    expect(lifecycleTone("starting")).toBe("pending");
    expect(lifecycleTone("idle")).toBe("neutral");
    expect(lifecycleTone("stopped")).toBe("neutral");
  });

  it("keeps the retired duplicate lifecycle labels out of the resources", () => {
    for (const key of ["layout.running", "layout.needsInput", "layout.pendingVerification", "layout.ready"]) {
      expect(en, key).not.toHaveProperty(key);
    }
  });
});
