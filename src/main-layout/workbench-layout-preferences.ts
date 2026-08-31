import type { SessionRuntimeSurfaceId } from "../session-workspace/session-surface-registry";
import type { WorkbenchDestination } from "./workbench-route";

const STORAGE_KEY = "vanehub.workbench.layout.v2";
const LEGACY_SESSION_SIDEBAR_WIDTH_KEY = "vanehub.session-sidebar.width.v1";

// 7.2: 256-400px is spec'd exactly (specs/main-layout-ui/spec.md, "Render wide session
// workspace"); 280px is tasks.md's own preferred default, not separately spec'd.
export const NAVIGATION_WIDTH_BOUNDS = { min: 256, max: 400, default: 280 };
export const INSPECTOR_WIDTH_BOUNDS = { min: 260, max: 480, default: 300 };
// No spec'd figure for the Runtime Panel exists yet, so this mirrors the inspector's own bounds —
// tall enough to read a log line or a shell prompt, short enough that Work never drops below its
// own documented minimum height (design.md Decision 7's "Resize or maximize" scenario).
export const RUNTIME_HEIGHT_BOUNDS = { min: 160, max: 640, default: 260 };

interface DestinationLayoutPreference {
  navigationWidth?: number;
  inspectorWidth?: number;
  preferredInspectorOpen?: boolean;
  runtimeHeight?: number;
  preferredRuntimeTab?: SessionRuntimeSurfaceId;
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

/**
 * Reads the pre-V2 sidebar width so a first V2 read still honors a returning user's choice.
 *
 * The absent-key check must happen before `Number(...)`: `Number(null)` is `0`, not `NaN`, so
 * `Number.isFinite` alone can't tell "key was never set" apart from "key is genuinely `0`" — the
 * former needs to fall through to `NAVIGATION_WIDTH_BOUNDS.default` in the caller, the latter
 * needs to clamp to `.min` like any other too-small stored value. Only visible once `default` and
 * `min` diverge (7.2: they used to both be 232) — before that, both paths produced the same
 * number, so no test could have told them apart.
 */
function readLegacySessionSidebarWidth(): number | undefined {
  if (typeof localStorage === "undefined") return undefined;
  const raw = localStorage.getItem(LEGACY_SESSION_SIDEBAR_WIDTH_KEY);
  if (raw === null) return undefined;
  const stored = Number(raw);
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
  runtimeHeight: number;
  /** Undefined means no surface has ever been opened; the Runtime Panel falls back to its own default. */
  preferredRuntimeTab: SessionRuntimeSurfaceId | undefined;
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
  const runtimeHeight = clamp(sessions?.runtimeHeight ?? RUNTIME_HEIGHT_BOUNDS.default, RUNTIME_HEIGHT_BOUNDS);
  return { navigationWidth, inspectorWidth, inspectorOpen, runtimeHeight, preferredRuntimeTab: sessions?.preferredRuntimeTab };
}

export function clampNavigationWidth(width: number): number {
  return clamp(width, NAVIGATION_WIDTH_BOUNDS);
}

export function clampInspectorWidth(width: number): number {
  return clamp(width, INSPECTOR_WIDTH_BOUNDS);
}

export function clampRuntimeHeight(height: number): number {
  return clamp(height, RUNTIME_HEIGHT_BOUNDS);
}
