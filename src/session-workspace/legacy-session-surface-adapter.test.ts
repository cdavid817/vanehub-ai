import { afterEach, describe, expect, it, vi } from "vitest";
import {
  legacySessionSurfaceAdapter,
  resolveLegacySessionSurface,
  type LegacySessionTabId,
} from "./legacy-session-surface-adapter";

describe("legacySessionSurfaceAdapter", () => {
  it("maps chat to work and terminal to terminal-history", () => {
    expect(legacySessionSurfaceAdapter("chat")).toBe("work");
    expect(legacySessionSurfaceAdapter("terminal")).toBe("terminal-history");
  });

  it("merges both documents and files onto the Files surface", () => {
    expect(legacySessionSurfaceAdapter("documents")).toBe("files");
    expect(legacySessionSurfaceAdapter("files")).toBe("files");
  });

  it("leaves every other legacy id unchanged", () => {
    for (const id of ["changes", "shell", "logs", "traces", "report"] as const) {
      expect(legacySessionSurfaceAdapter(id)).toBe(id);
    }
  });

  it("covers all nine legacy ids with no gaps", () => {
    const legacyIds: LegacySessionTabId[] = [
      "chat", "changes", "documents", "files", "terminal", "shell", "logs", "traces", "report",
    ];
    for (const id of legacyIds) {
      expect(typeof legacySessionSurfaceAdapter(id)).toBe("string");
    }
  });
});

describe("resolveLegacySessionSurface", () => {
  const originalError = console.error;
  afterEach(() => {
    console.error = originalError;
  });

  it("resolves a known legacy id the same way the static adapter does", () => {
    expect(resolveLegacySessionSurface("documents")).toBe("files");
    expect(resolveLegacySessionSurface("chat")).toBe("work");
  });

  it("returns null rather than guessing for an id with no target mapping", () => {
    console.error = vi.fn();
    expect(resolveLegacySessionSurface("onepiece-scratch")).toBeNull();
    expect(resolveLegacySessionSurface("")).toBeNull();
  });
});
