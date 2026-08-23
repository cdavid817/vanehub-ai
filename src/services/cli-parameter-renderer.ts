import type {
  CliArgumentSlot,
  CliLaunchScope,
  CliParameterDefinition,
  CliParameterPrimitive,
  CliParameterSelections,
} from "../types/cli-parameter";
import type { CliArgumentSegments, CliArgumentToken } from "../types/cli-parameter-profile";

// A port of the native render strategies, kept declarative for the same reason the native side is:
// no branch here may key on a parameter id. It exists only so the Web/mock adapter can show a
// preview without a native process; the desktop client always previews through the command.

function asBool(value: CliParameterPrimitive): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

function asText(value: CliParameterPrimitive): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function asTextList(value: CliParameterPrimitive): string[] | undefined {
  return Array.isArray(value) ? value : undefined;
}

/** TOML basic string, matching the native encoder character for character. */
export function tomlBasicString(value: string): string {
  let encoded = '"';
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (character === '"') encoded += '\\"';
    else if (character === "\\") encoded += "\\\\";
    else if (character === "\b") encoded += "\\b";
    else if (character === "\t") encoded += "\\t";
    else if (character === "\n") encoded += "\\n";
    else if (character === "\f") encoded += "\\f";
    else if (character === "\r") encoded += "\\r";
    else if (code < 0x20 || code === 0x7f)
      encoded += `\\u${code.toString(16).toUpperCase().padStart(4, "0")}`;
    else encoded += character;
  }
  return `${encoded}"`;
}

function renderTokens(
  definition: CliParameterDefinition,
  value: CliParameterPrimitive,
): string[] {
  const renderer = definition.renderer;
  switch (renderer.kind) {
    case "presence-flag":
      return asBool(value) === true ? [renderer.flag] : [];
    case "positive-negative-flag": {
      const flag = asBool(value);
      if (flag === true) return [renderer.positiveFlag];
      if (flag === false) return [renderer.negativeFlag];
      return [];
    }
    case "flag-value": {
      const text = asText(value);
      return text === undefined ? [] : [renderer.flag, text];
    }
    case "repeat-flag-value":
      return (asTextList(value) ?? []).flatMap((entry) => [renderer.flag, entry]);
    case "joined-list": {
      const entries = asTextList(value);
      if (!entries || entries.length === 0) return [];
      return [renderer.flag, entries.join(renderer.separator)];
    }
    case "config-key-value": {
      const encoded =
        renderer.encoding === "toml-string"
          ? mapDefined(asText(value), tomlBasicString)
          : mapDefined(asBool(value), (entry) => (entry ? "true" : "false"));
      return encoded === undefined ? [] : [renderer.flag, `${renderer.key}=${encoded}`];
    }
  }
}

function mapDefined<T, R>(value: T | undefined, project: (value: T) => R): R | undefined {
  return value === undefined ? undefined : project(value);
}

function slotOf(definition: CliParameterDefinition): CliArgumentSlot {
  return definition.renderer.slot;
}

/** Renders the user-editable half of a profile for one launch scope. Inherited entries produce no
 * token: inheritance means "let the provider decide", not "send the provider's default back". */
export function renderCliParameterSegments(
  definitions: CliParameterDefinition[],
  selections: CliParameterSelections,
  scope: CliLaunchScope,
): CliArgumentSegments {
  const segments: CliArgumentSegments = { global: [], invocation: [] };
  for (const definition of definitions) {
    if (definition.ownership !== "user-editable") continue;
    if (!definition.launchScopes.includes(scope)) continue;
    const selection = selections[definition.id];
    if (!selection || selection.state === "inherit") continue;
    const slot = slotOf(definition);
    for (const value of renderTokens(definition, selection.value)) {
      const token: CliArgumentToken = { value, parameterId: definition.id, segment: slot };
      segments[slot].push(token);
    }
  }
  return segments;
}

export function cliArgumentSegmentValues(segments: CliArgumentSegments): string[] {
  return [...segments.global, ...segments.invocation].map((token) => token.value);
}
