// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../../test/render";
import type { ProviderCredentialValidationResult } from "../../types/provider-credential-validation";
import { ProviderCredentialValidation } from "./provider-credential-validation";

describe("ProviderCredentialValidation", () => {
  it("discards a response after the provider configuration changes", async () => {
    let resolveRequest: ((result: ProviderCredentialValidationResult) => void) | undefined;
    const onValidate = vi.fn(() => new Promise<ProviderCredentialValidationResult>((resolve) => {
      resolveRequest = resolve;
    }));
    const rendered = renderWithAppProviders(
      <ProviderCredentialValidation onValidate={onValidate} resetKey={1} />,
    );

    await rendered.user.click(screen.getByRole("button", { name: "验证 API 密钥" }));
    rendered.rerender(<ProviderCredentialValidation onValidate={onValidate} resetKey={2} />);
    resolveRequest?.({ status: "valid", latencyMs: 10, httpStatus: 200 });

    await waitFor(() => expect(screen.getByRole("button", { name: "验证 API 密钥" })).toBeTruthy());
    expect(screen.queryByText("API 密钥有效。")).toBeNull();
  });
});
