export function requireHttpsExternalUrl(value: string): string {
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error("External URL is invalid.");
  }
  if (parsed.protocol !== "https:") throw new Error("Only HTTPS external URLs are allowed.");
  return parsed.toString();
}
