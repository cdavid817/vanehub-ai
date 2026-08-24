import type {
  CliParameterCategory,
  CliParameterDefinition,
  CliParameterSelections,
} from "../../types/cli-parameter";
import type {
  CliParameterDiagnostic,
  CliParameterErrorCode,
  CliParameterFieldView,
  CliParameterServiceError,
  CliParameterSupport,
} from "../../types/cli-parameter-profile";

const errorCodes: readonly CliParameterErrorCode[] = [
  "CLI_PARAMETER_UNKNOWN_AGENT",
  "CLI_PARAMETER_UNKNOWN_PARAMETER",
  "CLI_PARAMETER_INVALID_VALUE",
  "CLI_PARAMETER_DEPENDENCY_UNSATISFIED",
  "CLI_PARAMETER_CONFLICT",
  "CLI_PARAMETER_UNSUPPORTED_VERSION",
  "CLI_PARAMETER_REVISION_CONFLICT",
  "CLI_PARAMETER_CATALOG_MISMATCH",
  "CLI_PARAMETER_CATALOG_INVALID",
  "CLI_PARAMETER_REPOSITORY_FAILURE",
];

export const cliParameterCategories: readonly CliParameterCategory[] = [
  "model",
  "experience",
  "context",
  "runtime",
  "diagnostics",
];

export const cliParameterFilters = [
  "all",
  "modified",
  "warnings",
  "unsupported",
  "advanced",
] as const;

export type CliParameterFilter = (typeof cliParameterFilters)[number];

function isStringRecord(value: unknown): value is Record<string, string> {
  return (
    typeof value === "object" &&
    value !== null &&
    Object.values(value).every((entry) => typeof entry === "string")
  );
}

/** Recognizes the structured rejection both adapters produce. Anything else is an unknown transport
 * failure and stays generic — the page never reads prose to decide what went wrong. */
export function asCliParameterServiceError(error: unknown): CliParameterServiceError | null {
  if (typeof error !== "object" || error === null) return null;
  const candidate = error as {
    code?: unknown;
    agentId?: unknown;
    parameterId?: unknown;
    details?: unknown;
  };
  const code = errorCodes.find((known) => known === candidate.code);
  if (!code) return null;
  return {
    code,
    ...(typeof candidate.agentId === "string"
      ? { agentId: candidate.agentId as CliParameterServiceError["agentId"] }
      : {}),
    ...(typeof candidate.parameterId === "string" ? { parameterId: candidate.parameterId } : {}),
    ...(isStringRecord(candidate.details) ? { details: candidate.details } : {}),
  };
}

export function cliParameterErrorMessageKey(code: CliParameterErrorCode): string {
  return `cliParameters.error.${code}`;
}

export function cliParameterDiagnosticMessageKey(diagnostic: CliParameterDiagnostic): string {
  return `cliParameters.diagnostics.${diagnostic.code}`;
}

/** The flag a reader recognizes. Every renderer names one; the two-flag form shows its positive
 * side because that is what enabling the parameter emits. */
export function cliParameterDisplayFlag(definition: CliParameterDefinition): string {
  const renderer = definition.renderer;
  return renderer.kind === "positive-negative-flag" ? renderer.positiveFlag : renderer.flag;
}

export function cliParameterSearchText(
  definition: CliParameterDefinition,
  translate: (key: string) => string,
): string {
  return [
    cliParameterDisplayFlag(definition),
    definition.id,
    translate(definition.labelKey),
    translate(definition.descriptionKey),
    ...definition.options.flatMap((option) => [
      translate(option.labelKey),
      translate(option.descriptionKey),
    ]),
  ]
    .join(" ")
    .toLocaleLowerCase();
}

export function supportMessage(
  support: CliParameterSupport,
  translate: (key: string, values?: Record<string, string>) => string,
): string {
  switch (support.state) {
    case "supported":
      return translate("cliParameters.support.supported");
    case "not-installed":
      return translate("cliParameters.support.notInstalled");
    case "unknown-version":
      return translate("cliParameters.support.unknownVersion", {
        range: support.requiredRange ?? "",
      });
    case "unsupported-version":
      return translate("cliParameters.support.unsupportedVersion", {
        installed: support.installedVersion,
        range: support.requiredRange,
      });
    case "unsupported-platform":
      return translate("cliParameters.support.unsupportedPlatform", {
        platform: support.platform,
      });
  }
}

export function isUnsupported(support: CliParameterSupport): boolean {
  return support.state === "unsupported-version" || support.state === "unsupported-platform";
}

export interface CliParameterFieldFilterInput {
  field: CliParameterFieldView;
  dirty: boolean;
  diagnostics: readonly CliParameterDiagnostic[];
  query: string;
  filter: CliParameterFilter;
  scope: "chat" | "interactive";
  translate: (key: string) => string;
}

/** One place decides whether a field is on screen, so the rail counts and the list can never
 * disagree about what "modified" or "warnings" means. */
export function fieldMatches(input: CliParameterFieldFilterInput): boolean {
  const { field, dirty, diagnostics, query, filter, scope, translate } = input;
  if (!field.definition.launchScopes.includes(scope)) return false;
  if (query && !cliParameterSearchText(field.definition, translate).includes(query)) return false;
  const parameterDiagnostics = diagnostics.filter(
    (diagnostic) => diagnostic.parameterId === field.definition.id,
  );
  switch (filter) {
    case "all":
      return true;
    case "modified":
      return dirty;
    case "warnings":
      return parameterDiagnostics.some((diagnostic) => diagnostic.severity !== "info");
    case "unsupported":
      return isUnsupported(field.support);
    case "advanced":
      return field.definition.advanced;
  }
}

export function dependencyParameterIds(definition: CliParameterDefinition): string[] {
  return definition.dependencies.requiresAll.map((condition) => condition.parameterId);
}

/** A dependency is unmet when the parameter it names is still inherited. That is exactly the shape
 * the registry encodes, so the page does not restate provider rules of its own. */
export function unmetDependencies(
  definition: CliParameterDefinition,
  selections: CliParameterSelections,
): string[] {
  return definition.dependencies.requiresAll
    .filter((condition) => {
      const selection = selections[condition.parameterId];
      if (!selection || selection.state === "inherit") return true;
      if (condition.operator === "equals") return selection.value !== condition.value;
      if (condition.operator === "contains") {
        return !(
          Array.isArray(selection.value) &&
          typeof condition.value === "string" &&
          selection.value.includes(condition.value)
        );
      }
      return false;
    })
    .map((condition) => condition.parameterId);
}
