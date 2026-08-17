import { describe, expect, it } from "vitest";
import { TerminalReplayStore } from "./terminal-replay-store";

describe("TerminalReplayStore", () => {
  it("evicts the least recently used replay at the global byte limit", () => {
    const store = new TerminalReplayStore(8, 12);
    store.append("old", "12345678");
    store.append("recent", "abcdefgh");

    expect(store.read("old")).toBe("");
    expect(store.read("recent")).toBe("abcdefgh");
  });

  it("refreshes recency when a replay is read", () => {
    const store = new TerminalReplayStore(8, 16);
    store.append("first", "11111111");
    store.append("second", "22222222");
    expect(store.read("first")).toBe("11111111");

    store.append("third", "33333333");

    expect(store.read("second")).toBe("");
    expect(store.read("first")).toBe("11111111");
  });
});
