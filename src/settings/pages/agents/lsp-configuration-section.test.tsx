// @vitest-environment jsdom

import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { LspConfiguration, LspServerDiscovery } from "../../../types/lsp";
import { LspConfigurationSection } from "./lsp-configuration-section";
import { lspTestDescriptors } from "../../../test/lsp-fixtures";

const configuration: LspConfiguration = {
  enabled: false,
  languages: [
    {
      language: "rust",
      enabled: false,
      executableOverride: null,
      startupArguments: null,
      initializationOptions: {},
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: "C:/tools/typescript-language-server.exe",
      startupArguments: null,
      initializationOptions: { preferences: { includeCompletionsForModuleExports: true } },
    },
  ],
  descriptors: lspTestDescriptors(),
};

const discoveries: LspServerDiscovery[] = [
  {
    language: "rust",
    server: "rust_analyzer",
    availability: "available",
    executablePath: "C:/tools/rust-analyzer.exe",
    arguments: [],
    reasonCode: null,
  },
  {
    language: "typescript_javascript",
    server: "typescript_language_server",
    availability: "available",
    executablePath: "C:/tools/typescript-language-server.exe",
    arguments: ["--stdio"],
    reasonCode: null,
  },
];

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

describe("LspConfigurationSection", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("loads configuration and discovery exclusively through AgentService", async () => {
    const getLspConfiguration = vi.fn(async () => configuration);
    const discoverLspServers = vi.fn(async () => discoveries);
    const service = createAgentServiceDouble({ getLspConfiguration, discoverLspServers });
    renderWithAppProviders(<LspConfigurationSection service={service} />);

    const masterSwitch = await screen.findByRole("checkbox", { name: /^启用 LSP 集成/ });
    expect((masterSwitch as HTMLInputElement).checked).toBe(false);
    expect((screen.getByRole("checkbox", { name: "启用 Rust 语言服务器" }) as HTMLInputElement).checked).toBe(false);
    expect((screen.getByRole("checkbox", { name: "启用 TypeScript / JavaScript 语言服务器" }) as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText("C:/tools/rust-analyzer.exe")).toBeTruthy();
    expect(getLspConfiguration).toHaveBeenCalledOnce();
    expect(discoverLspServers).toHaveBeenCalledOnce();
  });

  it("shows initial loading, hides service errors, and retries both service queries", async () => {
    const pendingConfiguration = deferred<LspConfiguration>();
    const pendingDiscovery = deferred<LspServerDiscovery[]>();
    const getLspConfiguration = vi.fn(async () => configuration)
      .mockImplementationOnce(() => pendingConfiguration.promise);
    const discoverLspServers = vi.fn(async () => discoveries)
      .mockImplementationOnce(() => pendingDiscovery.promise);
    const service = createAgentServiceDouble({ getLspConfiguration, discoverLspServers });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    expect(screen.getByText("正在加载语言服务器设置…")).toBeTruthy();
    await act(async () => {
      pendingConfiguration.resolve(configuration);
      pendingDiscovery.reject(new Error("C:/private/configuration"));
    });

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("无法加载语言服务器设置。");
    expect(alert.textContent).not.toContain("private");
    await user.click(screen.getByRole("button", { name: "重试" }));

    expect(await screen.findByRole("checkbox", { name: /^启用 LSP 集成/ })).toBeTruthy();
    expect(getLspConfiguration).toHaveBeenCalledTimes(2);
    expect(discoverLspServers).toHaveBeenCalledTimes(2);
  });

  it("submits normalized switches, executable overrides, and initialization options", async () => {
    const saveLspConfiguration = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configuration,
      discoverLspServers: async () => discoveries,
      saveLspConfiguration,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    await user.click(await screen.findByRole("checkbox", { name: /^启用 LSP 集成/ }));
    await user.click(screen.getByRole("checkbox", { name: "启用 Rust 语言服务器" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Rust · 可执行文件覆盖路径" }), {
      target: { value: "C:/custom/rust-analyzer.exe" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Rust · 初始化选项" }), {
      target: { value: "{\"cargo\":{\"allTargets\":true}}" },
    });
    await user.click(screen.getByRole("button", { name: "保存 LSP 配置" }));

    await waitFor(() => expect(saveLspConfiguration).toHaveBeenCalledWith({
      enabled: true,
      languages: [
        {
          language: "rust",
          enabled: true,
          executableOverride: "C:/custom/rust-analyzer.exe",
          startupArguments: null,
          initializationOptions: { cargo: { allTargets: true } },
        },
        configuration.languages[1],
        // Described by the backend but absent from stored configuration, which is what a language
        // registered after this installation was set up looks like. It has to render and save with
        // defaults rather than be dropped.
        {
          language: "java",
          enabled: false,
          executableOverride: null,
          startupArguments: null,
          initializationOptions: {},
        },
      ],
      descriptors: lspTestDescriptors(),
    }));
    expect((await screen.findByRole("status")).textContent).toContain("LSP 配置已保存。");
  });

  it("rejects malformed initialization JSON without replacing persisted configuration", async () => {
    const saveLspConfiguration = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => configuration,
      discoverLspServers: async () => discoveries,
      saveLspConfiguration,
    });
    const { user } = renderWithAppProviders(<LspConfigurationSection service={service} />);

    const options = await screen.findByRole("textbox", { name: "Rust · 初始化选项" });
    fireEvent.change(options, {
      target: { value: "{broken" },
    });
    await user.click(screen.getByRole("button", { name: "保存 LSP 配置" }));

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("请输入有效的 JSON。");
    expect(options.getAttribute("aria-invalid")).toBe("true");
    expect(options.getAttribute("aria-describedby")).toContain(alert.id);
    expect(saveLspConfiguration).not.toHaveBeenCalled();
  });

  it("takes the override control's meaning from the descriptor, not from the language name", async () => {
    // Deliberately not "java". If the card branched on a language id this would render an
    // executable override, and the property the registry was built to have -- a second
    // install-directory language needing no frontend change -- would already be lost.
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => ({
        enabled: false,
        languages: [],
        descriptors: [{
          language: "elixir",
          server: "elixir_ls",
          supportedOnHost: true,
          defaultStartupArguments: [],
          overrideTarget: "install_directory" as const,
          prerequisite: "Erlang/OTP 26 or newer",
          distribution: null,
          installed: false,
        }],
      }),
      discoverLspServers: async () => [],
    });
    renderWithAppProviders(<LspConfigurationSection service={service} />);

    expect(await screen.findByRole("textbox", { name: /服务器安装目录/ })).toBeDefined();
    expect(screen.queryByRole("textbox", { name: /可执行文件覆盖路径/ })).toBeNull();
    // The prerequisite is the backend's string, rendered rather than mapped through a table the
    // frontend would have to extend for every new runtime.
    expect(screen.getByRole("note").textContent).toContain("Erlang/OTP 26 or newer");
  });
});
