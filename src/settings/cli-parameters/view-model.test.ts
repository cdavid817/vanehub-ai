import { describe, expect, it } from "vitest";
import {
  asCliParameterServiceError,
  cliParameterDisplayFlag,
  cliParameterErrorMessageKey,
  fieldMatches,
  supportMessage,
  unmetDependencies,
} from "./view-model";
import {
  cliParameterDefinitions,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "../../services/cli-parameter-registry";
import {
  cliArgumentSegmentValues,
  renderCliParameterSegments,
} from "../../services/cli-parameter-renderer";
import type { CliParameterFieldView } from "../../types/cli-parameter-profile";

const identity = (key: string) => key;

function fieldFor(agentId: "claude-code" | "codex-cli" | "gemini-cli", id: string): CliParameterFieldView {
  const definition = cliParameterDefinitions(agentId).find((entry) => entry.id === id);
  if (!definition) throw new Error(`${agentId}:${id} is missing from the registry`);
  return { definition, support: { state: "supported" }, optionSupport: {} };
}

describe("CLI parameter view model", () => {
  it("maps a structured field error by code without reading its prose", () => {
    // The rejection carries no English sentence at all: the page has nothing to regex even if it
    // wanted to.
    const rejection = { code: "CLI_PARAMETER_INVALID_VALUE", agentId: "codex-cli", parameterId: "model" };

    const parsed = asCliParameterServiceError(rejection);

    expect(parsed).not.toBeNull();
    expect(parsed?.parameterId).toBe("model");
    expect(cliParameterErrorMessageKey(parsed!.code)).toBe(
      "cliParameters.error.CLI_PARAMETER_INVALID_VALUE",
    );
  });

  it("refuses to recognize an unstructured failure as a field error", () => {
    expect(asCliParameterServiceError(new Error("Invalid value for CLI parameter: model"))).toBeNull();
    expect(asCliParameterServiceError({ code: "SOMETHING_ELSE" })).toBeNull();
    expect(asCliParameterServiceError("boom")).toBeNull();
  });

  it("names the positive flag of a tri-state rather than its negation", () => {
    expect(cliParameterDisplayFlag(fieldFor("claude-code", "chrome").definition)).toBe("--chrome");
  });

  it("hides a parameter that does not belong to the selected scope", () => {
    const chatOnly = fieldFor("claude-code", "bare");
    const shared = { dirty: false, diagnostics: [], query: "", filter: "all" as const, translate: identity };

    expect(fieldMatches({ ...shared, field: chatOnly, scope: "chat" })).toBe(true);
    expect(fieldMatches({ ...shared, field: chatOnly, scope: "interactive" })).toBe(false);
  });

  it("reports an unmet dependency using the registry's own rule", () => {
    const variant = cliParameterDefinitions("opencode").find((entry) => entry.id === "variant");
    expect(variant).toBeDefined();
    const inherited = defaultCliParameterSelections("opencode");

    expect(unmetDependencies(variant!, inherited)).toEqual(["model"]);
    expect(
      unmetDependencies(variant!, {
        ...inherited,
        model: { state: "value", value: "anthropic/claude-sonnet-4" },
      }),
    ).toEqual([]);
  });

  it("explains an unsupported version with both numbers", () => {
    const message = supportMessage(
      { state: "unsupported-version", installedVersion: "2.0.0", requiredRange: ">=2.1.181" },
      (key, values) => `${key}:${values?.installed ?? ""}:${values?.range ?? ""}`,
    );

    expect(message).toBe("cliParameters.support.unsupportedVersion:2.0.0:>=2.1.181");
  });
});

describe("CLI parameter preview scope and tokens", () => {
  it("differs between chat and interactive for a scope-specific parameter", () => {
    const definitions = editableCliParameterDefinitions("codex-cli");
    const selections = {
      ...defaultCliParameterSelections("codex-cli"),
      noAltScreen: { state: "value" as const, value: true },
      ephemeral: { state: "value" as const, value: true },
    };

    const chat = cliArgumentSegmentValues(renderCliParameterSegments(definitions, selections, "chat"));
    const interactive = cliArgumentSegmentValues(
      renderCliParameterSegments(definitions, selections, "interactive"),
    );

    expect(interactive).toContain("--no-alt-screen");
    expect(chat).not.toContain("--no-alt-screen");
    expect(chat).toContain("--ephemeral");
    expect(interactive).not.toContain("--ephemeral");
  });

  it("keeps a whitespace-bearing value as one argv token rather than a joined string", () => {
    const definitions = editableCliParameterDefinitions("gemini-cli");
    const segments = renderCliParameterSegments(
      definitions,
      {
        ...defaultCliParameterSelections("gemini-cli"),
        includeDirectories: { state: "value", value: ["C:/Program Files/app", "D:/work space"] },
      },
      "chat",
    );

    const tokens = cliArgumentSegmentValues(segments);
    expect(tokens).toContain("C:/Program Files/app");
    expect(tokens).toContain("D:/work space");
    // Each path is its own token, preceded by its own flag; nothing is comma- or space-joined.
    expect(tokens.filter((token) => token === "--include-directories")).toHaveLength(2);
    expect(tokens.some((token) => token.includes(","))).toBe(false);
  });
});
