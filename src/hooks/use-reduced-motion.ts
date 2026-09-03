import { useMediaQuery } from "./use-media-query";

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/**
 * Task 20.12: named wrapper around `useMediaQuery` for the one query this app conditions
 * component-level (not just CSS-level) behavior on, so call sites read as intent ("skip this
 * animation") rather than a raw media-query string repeated at every use.
 */
export function useReducedMotion(): boolean {
  return useMediaQuery(REDUCED_MOTION_QUERY);
}
