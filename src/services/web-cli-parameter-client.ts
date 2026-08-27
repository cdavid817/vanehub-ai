import type { CliParameterService } from "./cli-service";
import {
  cliParameterCatalogVersion,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "./cli-parameter-registry";
import { renderCliParameterSegments } from "./cli-parameter-renderer";
import { readWebMockStorage, writeWebMockStorage } from "./web-mock-storage";
import { nowIso } from "./web-mock-clock";
import { managedCliAgentIds, type ManagedCliAgentId } from "../types/agent";
import type { CliParameterSelections } from "../types/cli-parameter";
import type {
  CliParameterFieldView,
  CliParameterProfile,
  CliParameterServiceError,
} from "../types/cli-parameter-profile";

// Selections are catalog flag choices — no credential reaches browser storage, so this satisfies
// the `frontend-runtime-architecture` prohibition in "Honest Web/mock behavior".
//
// The mock is honest about what it cannot do: it has no CLI to detect, so every installation
// reports "not installed" and every field's support is `not-installed`. It never claims a version.

interface StoredProfile {
  selections: CliParameterSelections;
  revision: number;
  updatedAt: string | null;
}

type StoredProfiles = Partial<Record<ManagedCliAgentId, StoredProfile>>;

const cliParameterStorageKey = "vanehub.cli-parameter-profiles.v2";
const legacyCliParameterStorageKey = "vanehub.cli-parameter-profiles.v1";

// `null` means "nothing has been read or written yet", which is what triggers the one-time
// legacy migration. An empty object would mean "migrated, and nothing was there".
let memoryProfiles: StoredProfiles | null = null;

/** Mirrors the native definition-aware legacy conversion. `"default"` and `false` were v1's two
 * sentinels for "not set", but neither is a sentinel where the definition gives it a real meaning,
 * so both are converted against the definition rather than by string match. An entry that cannot be
 * read unambiguously is dropped, which leaves the parameter inherited. */
function convertLegacySelections(
  agentId: ManagedCliAgentId,
  legacy: Record<string, unknown>,
): CliParameterSelections {
  const selections: CliParameterSelections = {};
  for (const definition of editableCliParameterDefinitions(agentId)) {
    const value = legacy[definition.id];
    if (typeof value === "string") {
      const isRealOption = definition.options.some((option) => option.value === value);
      if (value !== "default" || isRealOption) selections[definition.id] = { state: "value", value };
    } else if (typeof value === "boolean") {
      const triState = definition.control === "tri-state";
      if (value || triState) selections[definition.id] = { state: "value", value };
    } else if (Array.isArray(value) && value.every((entry) => typeof entry === "string")) {
      if (value.length > 0) selections[definition.id] = { state: "value", value };
    }
  }
  return { ...defaultCliParameterSelections(agentId), ...selections };
}

/** One-time, read-side, non-destructive: the v1 key is left untouched so a downgrade still finds
 * its own data. */
function migrateLegacyProfiles(): StoredProfiles {
  const legacy = readWebMockStorage<Partial<Record<ManagedCliAgentId, Record<string, unknown>>>>(
    legacyCliParameterStorageKey,
    {},
  );
  const migrated: StoredProfiles = {};
  for (const agentId of managedCliAgentIds) {
    const stored = legacy[agentId];
    if (!stored) continue;
    migrated[agentId] = {
      selections: convertLegacySelections(agentId, stored),
      revision: 0,
      updatedAt: null,
    };
  }
  return migrated;
}

function readProfiles(): StoredProfiles {
  const stored = readWebMockStorage<StoredProfiles | null>(cliParameterStorageKey, memoryProfiles);
  if (stored) return stored;
  const migrated = migrateLegacyProfiles();
  memoryProfiles = migrated;
  return migrated;
}

function writeProfiles(value: StoredProfiles) {
  memoryProfiles = value;
  writeWebMockStorage(cliParameterStorageKey, value);
}

function serviceError(error: CliParameterServiceError): Error {
  // The desktop command rejects with this object; the mock throws the same shape so a page that
  // reads `code` works identically against both adapters.
  return Object.assign(new Error(error.code), error);
}

function requireAgentId(agentId: string): ManagedCliAgentId {
  const known = managedCliAgentIds.find((candidate) => candidate === agentId);
  if (!known) throw serviceError({ code: "CLI_PARAMETER_UNKNOWN_AGENT" });
  return known;
}

function fields(agentId: ManagedCliAgentId): CliParameterFieldView[] {
  return editableCliParameterDefinitions(agentId).map((definition) => ({
    definition,
    support: { state: "not-installed" },
    optionSupport: Object.fromEntries(
      definition.options.map((option) => [option.value, { state: "not-installed" } as const]),
    ),
  }));
}

function normalize(
  agentId: ManagedCliAgentId,
  selections: CliParameterSelections,
): CliParameterSelections {
  const known = new Set(editableCliParameterDefinitions(agentId).map((entry) => entry.id));
  for (const parameterId of Object.keys(selections)) {
    if (!known.has(parameterId)) {
      throw serviceError({ code: "CLI_PARAMETER_UNKNOWN_PARAMETER", agentId, parameterId });
    }
  }
  return { ...defaultCliParameterSelections(agentId), ...selections };
}

function profileOf(agentId: ManagedCliAgentId, stored: StoredProfile | undefined): CliParameterProfile {
  const selections = stored?.selections ?? defaultCliParameterSelections(agentId);
  const definitions = editableCliParameterDefinitions(agentId);
  return {
    agentId,
    catalogVersion: cliParameterCatalogVersion,
    revision: stored?.revision ?? 0,
    updatedAt: stored?.updatedAt ?? null,
    installation: { installed: false, runnable: false, conflict: false },
    fields: fields(agentId),
    selections,
    savedPreviews: {
      chat: renderCliParameterSegments(definitions, selections, "chat"),
      interactive: renderCliParameterSegments(definitions, selections, "interactive"),
    },
    diagnostics: [
      {
        code: "CLI_NOT_INSTALLED",
        severity: "info",
        agentId,
        messageKey: "cliParameters.diagnostics.CLI_NOT_INSTALLED",
        blocking: false,
        remediation: "open-cli-management",
      },
    ],
  };
}

/** Rejects a write whose caller was looking at a different revision or a different catalog. The
 * mock enforces this rather than skipping it, because a page that only ever runs against a
 * permissive adapter will not have its conflict handling exercised at all. */
function checkOptimisticTokens(
  agentId: ManagedCliAgentId,
  expectedRevision: number,
  catalogVersion: string,
  stored: StoredProfile | undefined,
) {
  if (catalogVersion !== cliParameterCatalogVersion) {
    throw serviceError({
      code: "CLI_PARAMETER_CATALOG_MISMATCH",
      agentId,
      details: {
        expectedCatalogVersion: catalogVersion,
        actualCatalogVersion: cliParameterCatalogVersion,
      },
    });
  }
  const actualRevision = stored?.revision ?? 0;
  if (expectedRevision !== actualRevision) {
    throw serviceError({
      code: "CLI_PARAMETER_REVISION_CONFLICT",
      agentId,
      details: {
        expectedRevision: String(expectedRevision),
        actualRevision: String(actualRevision),
      },
    });
  }
}

export const webCliParameterClient: CliParameterService = {
  async listCliParameterProfiles() {
    const stored = readProfiles();
    return managedCliAgentIds.map((agentId) => profileOf(agentId, stored[agentId]));
  },

  async previewCliParameterProfile(input) {
    const agentId = requireAgentId(input.agentId);
    if (input.catalogVersion !== cliParameterCatalogVersion) {
      throw serviceError({
        code: "CLI_PARAMETER_CATALOG_MISMATCH",
        agentId,
        details: {
          expectedCatalogVersion: input.catalogVersion,
          actualCatalogVersion: cliParameterCatalogVersion,
        },
      });
    }
    const selections = normalize(agentId, input.selections);
    return {
      agentId,
      catalogVersion: cliParameterCatalogVersion,
      scope: input.scope,
      ...(input.requestId === undefined ? {} : { requestId: input.requestId }),
      normalizedSelections: selections,
      segments: renderCliParameterSegments(
        editableCliParameterDefinitions(agentId),
        selections,
        input.scope,
      ),
      diagnostics: [],
    };
  },

  async saveCliParameterProfile(input) {
    const agentId = requireAgentId(input.agentId);
    const profiles = readProfiles();
    checkOptimisticTokens(agentId, input.expectedRevision, input.catalogVersion, profiles[agentId]);
    const next: StoredProfile = {
      selections: normalize(agentId, input.selections),
      revision: input.expectedRevision + 1,
      updatedAt: nowIso(),
    };
    writeProfiles({ ...profiles, [agentId]: next });
    return profileOf(agentId, next);
  },

  async resetCliParameterProfile(input) {
    const agentId = requireAgentId(input.agentId);
    const profiles = readProfiles();
    checkOptimisticTokens(agentId, input.expectedRevision, input.catalogVersion, profiles[agentId]);
    const next: StoredProfile = {
      selections: defaultCliParameterSelections(agentId),
      revision: input.expectedRevision + 1,
      updatedAt: nowIso(),
    };
    writeProfiles({ ...profiles, [agentId]: next });
    return profileOf(agentId, next);
  },
};
