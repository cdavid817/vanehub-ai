import type { WorkbenchDestination } from "./workbench-route";

const STORAGE_KEY = "vanehub.workbench.layout.v2";
const LEGACY_SESSION_SIDEBAR_WIDTH_KEY = "vanehub.session-sidebar.width.v1";

export const NAVIGATION_WIDTH_BOUNDS = { min: 232, max: 420, default: 232 };
export const INSPECTOR_WIDTH_BOUNDS = { min: 260, max: 480, default: 300 };

/**
 * `runtimeHeight`/`preferredRuntimeTab` from design.md Decision 3's shape are omitted here: the
 * Runtime Panel they describe belongs to the session workspace redesign (Decision 7), not this
 * pane-shell migration, and nothing produces a `RuntimePanelTabId` yet to type them against.
 */
interface DestinationLayoutPreference {
  navigationWidth?: number;
  inspectorWidth?: number;
  preferredInspectorOpen?: boolean;
}

export interface WorkbenchLayoutPreferencesV2 {
  version: 2;
  destination: Partial<Record<WorkbenchDestination, DestinationLayoutPreference>>;
}

function clamp(value: number, bounds: { min: number; max: number }): number {
  return Math.min(bounds.max, Math.max(bounds.min, Math.round(value)));
}

function emptyPreferences(): WorkbenchLayoutPreferencesV2 {
  return { version: 2, destination: {} };
}

function isDestinationLayoutPreference(value: unknown): value is DestinationLayoutPreference {
  return typeof value === "object" && value !== null;
}

/** Malformed storage never blocks startup — any shape mismatch falls back to empty preferences. */
function readWorkbenchLayoutPreferences(): WorkbenchLayoutPreferencesV2 {
  if (typeof localStorage === "undefined") return emptyPreferences();
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return emptyPreferences();
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || (parsed as { version?: unknown }).version !== 2) {
      return emptyPreferences();
    }
    const destination = (parsed as { destination?: unknown }).destination;
    if (typeof destination !== "object" || destination === null) return emptyPreferences();
    const result: WorkbenchLayoutPreferencesV2 = emptyPreferences();
    for (const [key, value] of Object.entries(destination)) {
      if (isDestinationLayoutPreference(value)) result.destination[key as WorkbenchDestination] = value;
    }
    return result;
  } catch {
    return emptyPreferences();
  }
}

function writeWorkbenchLayoutPreferences(preferences: WorkbenchLayoutPreferencesV2) {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences));
}

/** Reads the pre-V2 sidebar width so a first V2 read still honors a returning user's choice. */
function readLegacySessionSidebarWidth(): number | undefined {
  if (typeof localStorage === "undefined") return undefined;
  const stored = Number(localStorage.getItem(LEGACY_SESSION_SIDEBAR_WIDTH_KEY));
  return Number.isFinite(stored) ? stored : undefined;
}

export function patchDestinationLayoutPreference(
  destination: WorkbenchDestination,
  patch: Partial<DestinationLayoutPreference>,
): void {
  const preferences = readWorkbenchLayoutPreferences();
  preferences.destination[destination] = { ...preferences.destination[destination], ...patch };
  writeWorkbenchLayoutPreferences(preferences);
}

export interface SessionsLayoutInitialState {
  navigationWidth: number;
  inspectorWidth: number;
  inspectorOpen: boolean;
}

/**
 * The one-time legacy migration design.md calls for: a V2 width, once present, always wins over
 * the legacy key — this only reads the legacy key for readers who have never resized under V2.
 */
export function readInitialSessionsLayout(): SessionsLayoutInitialState {
  const preferences = readWorkbenchLayoutPreferences();
  const sessions = preferences.destination.sessions;
  const navigationWidth = clamp(
    sessions?.navigationWidth ?? readLegacySessionSidebarWidth() ?? NAVIGATION_WIDTH_BOUNDS.default,
    NAVIGATION_WIDTH_BOUNDS,
  );
  const inspectorWidth = clamp(sessions?.inspectorWidth ?? INSPECTOR_WIDTH_BOUNDS.default, INSPECTOR_WIDTH_BOUNDS);
  // Closed, not open: at every tier narrower than Wide the inspector presents as a modal Sheet
  // (design.md Decision 3), and a full-viewport backdrop on a reader's very first paint -- on an
  // entirely ordinary laptop-width window -- is a worse first impression than an easy-to-find
  // toggle they have not pressed yet.
  const inspectorOpen = sessions?.preferredInspectorOpen ?? false;
  return { navigationWidth, inspectorWidth, inspectorOpen };
}

export function clampNavigationWidth(width: number): number {
  return clamp(width, NAVIGATION_WIDTH_BOUNDS);
}

export function clampInspectorWidth(width: number): number {
  return clamp(width, INSPECTOR_WIDTH_BOUNDS);
}
