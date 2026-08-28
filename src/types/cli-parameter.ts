import type { ManagedCliAgentId } from "./agent";

export type CliParameterCategory =
  | "model"
  | "experience"
  | "context"
  | "runtime"
  | "diagnostics";

/** Only `user-editable` definitions reach the settings page. The rest stay registry-owned so the
 * policy and runtime paths can render them from their owning source. */
export type CliParameterOwnership = "user-editable" | "policy-governed" | "runtime-reserved";

export type CliParameterMaturity = "stable" | "preview" | "experimental" | "deprecated";

export type CliParameterControl =
  | "enum"
  | "boolean-flag"
  | "tri-state"
  | "multi-enum"
  | "custom-text"
  | "ordered-string-list"
  | "path-list";

export type CliParameterRisk = "normal" | "warning";

export type CliLaunchScope = "interactive" | "chat";

export type CliParameterPlatform = "windows" | "macos" | "linux";

/** `global` precedes a provider subcommand; `invocation` belongs to the invocation grammar. */
export type CliArgumentSlot = "global" | "invocation";

export type CliParameterPrimitive = string | boolean | string[];

/** Inheritance is a distinct state, so a provider value literally named `default` stays
 * representable and renderable. */
export type CliParameterSelection =
  | { state: "inherit" }
  | { state: "value"; value: CliParameterPrimitive };

export type CliParameterSelections = Record<string, CliParameterSelection>;

export interface CliParameterCompatibility {
  minVersion?: string;
  maxVersion?: string;
  platforms: CliParameterPlatform[];
}

export type CliParameterOrdering = "catalog" | "user";

export interface CliParameterConstraints {
  maxLength?: number;
  pattern?: string;
  maxItems?: number;
  itemMaxLength?: number;
  itemPattern?: string;
  dedupe: boolean;
  exclusiveValues: string[];
  ordering?: CliParameterOrdering;
}

export type CliConditionOperator = "equals" | "not-inherit" | "contains";

export interface CliParameterCondition {
  parameterId: string;
  operator: CliConditionOperator;
  value?: string | boolean;
}

export interface CliParameterDependencies {
  requiresAll: CliParameterCondition[];
  conflictsWith: string[];
}

export interface CliParameterOption {
  value: string;
  labelKey: string;
  descriptionKey: string;
  compatibility?: CliParameterCompatibility;
}

export type CliParameterRenderer =
  | { kind: "presence-flag"; flag: string; slot: CliArgumentSlot }
  | {
      kind: "positive-negative-flag";
      positiveFlag: string;
      negativeFlag: string;
      slot: CliArgumentSlot;
    }
  | { kind: "flag-value"; flag: string; slot: CliArgumentSlot }
  | { kind: "repeat-flag-value"; flag: string; slot: CliArgumentSlot }
  | { kind: "joined-list"; flag: string; separator: string; slot: CliArgumentSlot }
  | {
      kind: "config-key-value";
      flag: string;
      key: string;
      encoding: "toml-string" | "toml-boolean";
      slot: CliArgumentSlot;
    };

export interface CliParameterDefinition {
  id: string;
  agentId: ManagedCliAgentId;
  category: CliParameterCategory;
  ownership: CliParameterOwnership;
  maturity: CliParameterMaturity;
  control: CliParameterControl;
  labelKey: string;
  descriptionKey: string;
  defaultSelection: CliParameterSelection;
  launchScopes: CliLaunchScope[];
  risk: CliParameterRisk;
  advanced: boolean;
  options: CliParameterOption[];
  renderer: CliParameterRenderer;
  constraints: CliParameterConstraints;
  compatibility: CliParameterCompatibility;
  dependencies: CliParameterDependencies;
  diagnostics: string[];
}

export interface CliParameterCatalogAgent {
  agentId: ManagedCliAgentId;
  parameters: CliParameterDefinition[];
}

export interface CliParameterCatalog {
  catalogVersion: string;
  selectionSchemaVersion: number;
  agents: CliParameterCatalogAgent[];
}

// Selection helpers, the category/scope value lists, and the renderer accessors land with their
// first consumer in `upgrade-cli-parameter-management` sections 10-12. Keeping this module
// type-only until then avoids shipping untested runtime code.
