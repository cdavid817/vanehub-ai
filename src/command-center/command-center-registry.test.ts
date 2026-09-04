import { describe, expect, it } from "vitest";
import { COMMANDS, SEARCH_PROVIDERS } from "./command-center-registry";

describe("command center registry", () => {
  it("aggregates all four shipped search providers, each with a unique id", () => {
    expect(SEARCH_PROVIDERS.map((provider) => provider.id).sort()).toEqual(["goals", "projects", "runs", "sessions"]);
  });

  it("supports exactly session/project/run/goal — work-item/evaluation have no provider yet", () => {
    const supportedScopes = ["session", "project", "run", "goal", "work-item", "evaluation"] as const;
    const supported = supportedScopes.filter((scope) => SEARCH_PROVIDERS.some((provider) => provider.supports(scope)));
    expect(supported.sort()).toEqual(["goal", "project", "run", "session"]);
  });

  it("aggregates every destination and contextual command with no duplicate id", () => {
    expect(COMMANDS.length).toBeGreaterThan(0);
    expect(new Set(COMMANDS.map((command) => command.id)).size).toBe(COMMANDS.length);
  });
});
