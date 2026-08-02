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
