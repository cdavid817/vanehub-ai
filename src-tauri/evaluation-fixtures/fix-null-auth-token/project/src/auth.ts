export function authorizationHeader(token: string | null): string | undefined {
  return token === null ? undefined : `Bearer ${token}`;
}
