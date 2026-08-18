// Single entry point for the Web/mock adapter's browser-storage access. Callers keep an in-memory
// fallback and pass it in, so a runtime without `localStorage` (SSR, jsdom without storage) and a
// corrupted entry both degrade to the same value instead of throwing.
//
// `frontend-runtime-architecture` / "Honest Web/mock behavior" forbids plaintext secrets here: only
// callers whose payload carries no credential may route through this module.
export function readWebMockStorage<T>(key: string, fallback: T): T {
  if (typeof localStorage === "undefined") return fallback;
  const raw = localStorage.getItem(key);
  if (!raw) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function writeWebMockStorage(key: string, value: unknown): void {
  if (typeof localStorage !== "undefined") localStorage.setItem(key, JSON.stringify(value));
}
