import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { refreshSshConnections } from "./ssh-connection-query";

describe("SSH connection query refresh", () => {
  it("invalidates the connection list when a mutation settles", async () => {
    const queryClient = new QueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");

    await refreshSshConnections(queryClient);

    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["ssh-connections"] });
  });
});
