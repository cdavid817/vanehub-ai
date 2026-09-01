// @vitest-environment jsdom

import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import { LspWorkspaceTrustPanel } from "./lsp-workspace-trust-panel";

describe("LspWorkspaceTrustPanel Revoke trust requires confirmation (task 12.14)", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("does not revoke trust until the confirmation is accepted, and revokes it once it is", async () => {
    const updateLspWorkspaceTrust = vi.fn(async (update: { canonicalRoot: string; trusted: boolean }) => ({
      canonicalRoot: update.canonicalRoot,
      trusted: update.trusted,
      revision: 2,
    }));
    const service = createAgentServiceDouble({
      listLspWorkspaceTrust: async () => [{ canonicalRoot: "D:/projects/vanehub", trusted: true, revision: 1 }],
      updateLspWorkspaceTrust,
    });
    renderWithAppProviders(<LspWorkspaceTrustPanel service={service} />);

    const revoke = await screen.findByRole("button", { name: /撤销信任/ }, { timeout: 10_000 });
    fireEvent.click(revoke);

    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("D:/projects/vanehub");
    expect(updateLspWorkspaceTrust).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(updateLspWorkspaceTrust).not.toHaveBeenCalled();

    fireEvent.click(revoke);
    const secondDialog = await screen.findByRole("dialog");
    fireEvent.click(within(secondDialog).getByRole("button", { name: "确认" }));
    await waitFor(() => expect(updateLspWorkspaceTrust).toHaveBeenCalledWith({
      canonicalRoot: "D:/projects/vanehub",
      trusted: false,
    }));
  });
});
