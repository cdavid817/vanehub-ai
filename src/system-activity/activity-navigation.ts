import type { ActivityNavigationKind } from "./activity-contracts";
import { decodeActivityNavigation } from "./activity-payload-decoder";

export interface ActivityNavigationDestination {
  readonly mode: "view";
  readonly detailKind: ActivityNavigationKind;
  readonly stableId: string;
  readonly childId?: string | null;
}

export type ActivityNavigator = (destination: ActivityNavigationDestination) => void;

export function resolveActivityNavigation(input: unknown): ActivityNavigationDestination | null {
  const navigation = decodeActivityNavigation(input);
  if (!navigation) return null;
  return Object.freeze({
    mode: "view" as const,
    detailKind: navigation.kind,
    stableId: navigation.stableId,
    ...(navigation.childId === undefined ? {} : { childId: navigation.childId }),
  });
}

export function openActivityNavigation(input: unknown, navigate: ActivityNavigator): boolean {
  const destination = resolveActivityNavigation(input);
  if (!destination) return false;
  navigate(destination);
  return true;
}
