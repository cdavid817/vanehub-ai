# 安装并认证 CLI

**状态：已交付——桌面端设置。**

VaneHub AI 驱动已经安装的 Coding Agent CLI。认证仍由各 Provider 的 CLI 管理；VaneHub AI 不会要求输入 Provider 密码。

## 前置条件

- Node.js 22+ 与 npm
- VaneHub AI 或本仓库的开发环境
- 至少一个受支持的 CLI，以及相应订阅或 API 凭据

安装一个 Provider：

```powershell
npm install -g @anthropic-ai/claude-code
```

```powershell
npm install -g @openai/codex
```

Gemini CLI 与 OpenCode 可按各自官方说明安装。

先在普通终端中运行所选命令并完成认证：

```powershell
claude
```

```powershell
codex
```

确认 CLI 能够接受提示词后，再通过 VaneHub AI 打开。Provider 凭据仍保存在 CLI 自己的存储中。

## Web 预览

**状态：仅 Web/mock。** 浏览器预览展示确定性的可用性与执行 fixture，不会检测或认证本地 CLI。
