// The Web/mock adapter stamps simulated records with real wall-clock time so relative-time UI
// renders the same way it does on desktop. Shared so extracted `web-*` modules read one clock.
export function nowIso() {
  return new Date().toISOString();
}

export function daysAgoIso(days: number) {
  const value = new Date();
  value.setDate(value.getDate() - days);
  return value.toISOString();
}
