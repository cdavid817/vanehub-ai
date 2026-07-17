import { describe, expect, it } from "vitest";
import { ServiceError } from "../services/service-error";
import { workspaceErrorKey } from "./workspace-error";

describe("workspace error localization", () => {
  it("maps normalized service codes without exposing native messages", () => {
    expect(workspaceErrorKey(new ServiceError("validation", "native details"))).toBe("sessionTabs.error.validation");
    expect(workspaceErrorKey(new ServiceError("not-found", "native details"))).toBe("sessionTabs.error.notFound");
    expect(workspaceErrorKey(new ServiceError("unsupported-runtime", "native details"))).toBe("sessionTabs.error.unsupported");
    expect(workspaceErrorKey(new Error("sensitive native details"))).toBe("sessionTabs.error.runtime");
  });
});
