export function formatAppDateTime(
  value: string | number | Date,
  language: string,
  options: Intl.DateTimeFormatOptions,
): string {
  return new Intl.DateTimeFormat(language, options).format(new Date(value));
}

export function formatAppNumber(
  value: number,
  language: string,
  options?: Intl.NumberFormatOptions,
): string {
  return new Intl.NumberFormat(language, options).format(value);
}

const millisecondsPerDay = 24 * 60 * 60 * 1000;

/**
 * Locale-native weekday names ordered to match `Date#getDay()`/`getUTCDay()` (index 0 = Sunday),
 * derived from `Intl.DateTimeFormat` instead of a hand-maintained translated array per locale so
 * the names can't drift out of sync with the platform's own locale data.
 */
export function formatAppWeekdayNames(
  language: string,
  options: Intl.DateTimeFormatOptions = { weekday: "short" },
): string[] {
  const formatter = new Intl.DateTimeFormat(language, { ...options, timeZone: "UTC" });
  const today = new Date();
  const sunday = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - today.getUTCDay());
  return Array.from({ length: 7 }, (_, weekday) => formatter.format(new Date(sunday + weekday * millisecondsPerDay)));
}
