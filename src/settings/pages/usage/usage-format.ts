export function formatUsageNumber(value: number, language: string) {
  return new Intl.NumberFormat(language, { maximumFractionDigits: 1 }).format(value);
}

export function formatUsageDate(value: string, language: string) {
  return new Intl.DateTimeFormat(language, { month: "short", day: "numeric" }).format(
    new Date(`${value}T00:00:00`),
  );
}

export function formatGeneratedAt(value: string | undefined, language: string) {
  if (!value) return null;
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
