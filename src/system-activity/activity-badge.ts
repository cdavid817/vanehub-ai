export function formatActivityUnreadBadge(count: number): string {
  const bounded = Math.max(0, Math.trunc(Number.isFinite(count) ? count : 0));
  return bounded > 99 ? "99+" : String(bounded);
}
