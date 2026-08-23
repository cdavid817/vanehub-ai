import type { CliParameterDefinition } from "../../types/cli-parameter";
import type {
  CliParameterErrorCode,
  CliParameterServiceError,
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

/** Recognizes the structured rejection both adapters produce. Anything else is an unknown
 * transport failure and stays generic — the page never reads prose to decide what went wrong. */
export function asCliParameterServiceError(error: unknown): CliParameterServiceError | null {
  if (typeof error !== "object" || error === null) return null;
  const candidate = error as { code?: unknown; agentId?: unknown; parameterId?: unknown; details?: unknown };
  if (typeof candidate.code !== "string") return null;
  const code = errorCodes.find((known) => known === candidate.code);
  if (!code) return null;
  return {
    code,
    ...(typeof candidate.agentId === "string" ? { agentId: candidate.agentId as CliParameterServiceError["agentId"] } : {}),
    ...(typeof candidate.parameterId === "string" ? { parameterId: candidate.parameterId } : {}),
    ...(isStringRecord(candidate.details) ? { details: candidate.details } : {}),
  };
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return (
    typeof value === "object" &&
    value !== null &&
    Object.values(value).every((entry) => typeof entry === "string")
  );
}

export function cliParameterErrorMessageKey(code: CliParameterErrorCode): string {
  return `cliParameters.error.${code}`;
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
