// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../test/render";
import type { ArtifactDetail } from "../types/builtin-tools";
import { ArtifactPanel } from "./artifact-panel";

const artifact: ArtifactDetail = {
  id: "artifact-1",
  displayName: "result.txt",
  mediaType: "text/plain",
  sizeBytes: 5,
  contentHash: "sha256:result",
  integrity: "verified",
  createdAt: "2026-08-14T00:00:00Z",
  expiresAt: "2026-09-14T00:00:00Z",
  simulated: false,
  producerOperationId: "operation-1",
  provenance: ["web_fetch", "https://example.com"],
  publishedAt: null,
  publicationUrl: null,
  limitations: [],
};

function artifactMethods(detail: ArtifactDetail) {
  return {
    listArtifacts: async () => ({ items: [detail], nextCursor: null }),
    getArtifact: async () => detail,
    readArtifact: async () => ({
      artifactId: detail.id,
      offset: 0,
      bytesBase64: "aGVsbG8=",
      nextOffset: null,
      contentHash: detail.contentHash,
    }),
  };
}

function serviceFor(detail: ArtifactDetail) {
  return createAgentServiceDouble(artifactMethods(detail));
}

describe("ArtifactPanel", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("shows provenance and preview, then requires explicit publication acknowledgement", async () => {
    const publishArtifact = vi.fn(async () => ({ ...artifact, publishedAt: "2026-08-14T01:00:00Z" }));
    const downloadArtifact = vi.fn(async () => ({ path: "hidden", contentHash: artifact.contentHash }));
    const service = createAgentServiceDouble({
      ...artifactMethods(artifact),
      publishArtifact,
      downloadArtifact,
    });
    const { user } = renderWithAppProviders(<ArtifactPanel service={service} sessionId="session-1" />);

    await user.click(await screen.findByRole("button", { name: /result\.txt/ }));
    expect(await screen.findByText("web_fetch · https://example.com")).toBeTruthy();
    expect(await screen.findByText("hello")).toBeTruthy();
    const publish = screen.getByRole("button", { name: "发布" });
    expect((publish as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("checkbox"));
    await user.click(publish);
    await waitFor(() => expect(publishArtifact).toHaveBeenCalledWith({
      artifactId: artifact.id,
      expectedContentHash: artifact.contentHash,
      acknowledgement: true,
    }));
    await user.click(screen.getByRole("button", { name: "下载" }));
    expect(downloadArtifact).toHaveBeenCalledWith({
      artifactId: artifact.id,
      expectedContentHash: artifact.contentHash,
    });
    expect(screen.queryByText("hidden")).toBeNull();
  });

  it("disables effects when integrity is not verified", async () => {
    const service = serviceFor({ ...artifact, integrity: "failed" });
    const { user } = renderWithAppProviders(<ArtifactPanel service={service} sessionId="session-1" />);

    await user.click(await screen.findByRole("button", { name: /result\.txt/ }));
    expect(await screen.findByRole("alert")).toBeTruthy();
    await user.click(screen.getByRole("checkbox"));
    expect((screen.getByRole("button", { name: "发布" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "下载" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
