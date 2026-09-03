# 本地扩展

本地扩展的安装、启用与禁用，以及内置的产品集成与就绪检测。

同在设置中心配置、但各有专章的：[MCP 服务器](mcp.md)、[Prompt Hook](prompt-hooks.md)、[Skill 管理](skill-management.md)、[Agent 与 CLI 配置](agent-configuration.md)。

## 扩展能力

**设置 → 扩展能力**里装的是**本地多模态 AI 能力**，不是通用插件。首版每种能力提供一个内置白名单框架：

| 能力 | 框架 | 运行时 | 本地端口 | 预计磁盘占用 |
| --- | --- | --- | --- | --- |
| **OCR 文字识别** | PaddleOCR | Python 3.10+ | 9875 | **~1800 MB** |
| **语音识别** | faster-whisper | Python 3.10+ | 9876 | **~900 MB** |
| **语音合成** | sherpa-onnx | Python 3.10+ | — | — |

**装之前先看两件事**：需要本机有 Python 3.10+，以及**磁盘占用不小**——PaddleOCR 接近 1.8 GB。每个框架卡片上都有「安装要求」可展开查看。

页面顶部有**已安装 / 运行中 / 异常**三个计数，异常时到操作日志里查原因。

![设置中的扩展能力页面，PaddleOCR 与 faster-whisper 框架卡片](assets/screenshots/extensions-zh-CN.png)

## 插件集成

**设置 → 插件集成**管理内置产品集成与就绪检测——注意它**不安装第三方插件包**。首版内置 GitHub 一个集成，检测本机 `gh` 的认证状态。五种状态的含义、启用步骤与 Web 模式限制，见[插件集成](plugin-integration.md)。

## 注意事项与限制

- **全部仅桌面端可用**。
- **扩展能力不改写各 CLI 自己的配置文件**，绑定通过启动参数与中继实现。
