// @vitest-environment jsdom

import { act, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { LspConfiguration, LspLanguageDescriptor } from "../../../types/lsp";
import { LspConfigurationSection } from "./lsp-configuration-section";

// Deliberately not "java". Java is the only language the registry distributes today, so a test
// written against it cannot tell descriptor-driven rendering apart from a hardcoded branch -- and
// the second distributed language would then arrive to a settings page that ignores it.
function elixir(overrides: Partial<LspLanguageDescriptor> = {}): LspLanguageDescriptor {
  return {
    language: "elixir",
    server: "elixir_ls",
    supportedOnHost: true,
    defaultStartupArguments: [],
    overrideTarget: "install_directory",
    prerequisite: "Erlang/OTP 26 or newer",
    distribution: { verified: false },
    installed: false,
    ...overrides,
  };
}

function configurationWith(descriptor: LspLanguageDescriptor): LspConfiguration {
  return { enabled: true, languages: [], descriptors: [descriptor] };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

describe("LSP managed installation", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("warns that the download is unverified before anything is downloaded", async () => {
    const installLspServer = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configurationWith(elixir()),
      discoverLspServers: async () => [],
      installLspServer,
    });
    renderWithAppProviders(<LspConfigurationSection service={service} />);

    const install = await screen.findByRole("button", { name: /· 安装服务器$/ });
    expect(screen.getByText(/未做校验和验证/)).toBeTruthy();
    expect(installLspServer).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /移除服务器/ })).toBeNull();
    expect(install.hasAttribute("disabled")).toBe(false);
  });

  it("installs through the service and reflects the backend's own installed state", async () => {
    const installLspServer = vi.fn(async () => undefined);
    // Second read answers what the backend would after a successful install; the card must take
    // "installed" from there rather than from having just clicked the button.
    const getLspConfiguration = vi.fn(async () => configurationWith(elixir({ installed: true })))
      .mockImplementationOnce(async () => configurationWith(elixir()));
    const service = createAgentServiceDouble({
      getLspConfiguration,
      discoverLspServers: async () => [],
      installLspServer,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    await user.click(await screen.findByRole("button", { name: /· 安装服务器$/ }));

    await waitFor(() => expect(installLspServer).toHaveBeenCalledWith("elixir"));
    expect(await screen.findByRole("button", { name: /· 移除服务器$/ })).toBeTruthy();
    expect(screen.getByText("由 VaneHub AI 安装。")).toBeTruthy();
    expect(getLspConfiguration).toHaveBeenCalledTimes(2);
  });

  it("removes through the service without touching any other language's state", async () => {
    const uninstallLspServer = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configurationWith(elixir({ installed: true })),
      discoverLspServers: async () => [],
      uninstallLspServer,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    await user.click(await screen.findByRole("button", { name: /· 移除服务器$/ }));

    await waitFor(() => expect(uninstallLspServer).toHaveBeenCalledWith("elixir"));
  });

  it("reports a refused install and leaves the button usable again", async () => {
    const installLspServer = vi.fn(async () => {
      throw new Error("install_refused");
    });
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configurationWith(elixir()),
      discoverLspServers: async () => [],
      installLspServer,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    await user.click(await screen.findByRole("button", { name: /· 安装服务器$/ }));

    expect(await screen.findByText("下载或归档被某项安全限制拒绝。")).toBeTruthy();
    // A control stuck in "处理中…" reads as a download still running when none is.
    const install = screen.getByRole("button", { name: /· 安装服务器$/ });
    expect(install.hasAttribute("disabled")).toBe(false);
    expect(install.textContent).toBe("安装服务器");
  });

  it("keeps each language's busy state its own while another install is running", async () => {
    // Two distributed languages, which the registry does not have yet -- and that is the point.
    // With a single busy slot, starting the second install cleared the first card's state and
    // re-enabled its button while its download was still going.
    const elixirGate = deferred<void>();
    const installLspServer = vi.fn(async (language: string) => {
      if (language === "elixir") await elixirGate.promise;
    });
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => ({
        enabled: true,
        languages: [],
        descriptors: [elixir(), elixir({ language: "gleam", server: "gleam_lsp" })],
      }),
      discoverLspServers: async () => [],
      installLspServer,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    const [elixirInstall, gleamInstall] = await screen.findAllByRole("button", {
      name: /· 安装服务器$/,
    });
    await user.click(elixirInstall!);
    await user.click(gleamInstall!);

    // Gleam's install resolved immediately; Elixir's is still held open, so its button must stay
    // disabled and its label must still read as working.
    await waitFor(() => expect(gleamInstall!.hasAttribute("disabled")).toBe(false));
    expect(elixirInstall!.hasAttribute("disabled")).toBe(true);
    expect(elixirInstall!.textContent).toBe("处理中…");

    await act(async () => {
      elixirGate.resolve();
    });
    await waitFor(() => expect(elixirInstall!.hasAttribute("disabled")).toBe(false));
  });

  it("offers no managed installation for a language the backend does not distribute", async () => {
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configurationWith(elixir({ distribution: null })),
      discoverLspServers: async () => [],
    });
    renderWithAppProviders(<LspConfigurationSection service={service} />);

    expect(await screen.findByRole("textbox", { name: /服务器安装目录/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /安装服务器/ })).toBeNull();
    expect(screen.queryByText(/未做校验和验证/)).toBeNull();
  });
});
