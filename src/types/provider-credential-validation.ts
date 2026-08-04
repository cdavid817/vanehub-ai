export type ProviderCredentialValidationStatus =
  | "valid"
  | "invalid-credential"
  | "configuration-rejected"
  | "rate-limited"
  | "provider-unavailable"
  | "unsupported"
  | "inconclusive";

export interface ProviderCredentialValidationResult {
  status: ProviderCredentialValidationStatus;
  latencyMs: number;
  httpStatus: number | null;
}
