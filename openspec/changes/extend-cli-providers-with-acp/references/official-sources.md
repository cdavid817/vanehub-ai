# 官方来源与核验说明

核验日期：2026-09-06。下列资料用于查证命令入口及产品边界，未下载或运行其中安装脚本，未执行真实 CLI。URL 是事实来源，不是可直接作为可信安装模板的执行授权。

## CLI

| 编号 | 官方页面 | 本包采用的事实 |
| --- | --- | --- |
| S01 | [Qwen 配置](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) | `--acp` 入口；参数按版本核对 |
| S02 | [Qwen 部署](https://qwenlm.github.io/qwen-code-docs/en/developers/development/deployment/) | npm 包 `@qwen-code/qwen-code` |
| S03 | [Qwen 安装概览](https://qwenlm.github.io/qwen-code-docs/en/index) | standalone installer 可能 fallback npm，需审查来源语义 |
| S04 | [Kimi 入门](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started.html) | 当前 npm 包与不同安装方式 |
| S05 | [Kimi 迁移](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/migration.html) | 当前与旧 Python/uv 发行形态迁移边界 |
| S06 | [Kimi ACP](https://www.kimi.com/code/docs/en/kimi-code-cli/reference/kimi-acp.html) | `kimi acp` |
| S07 | [Kimi 命令参考](https://www.kimi.com/code/docs/en/kimi-code-cli/reference/kimi-command.html) | print 无人工审批，静态 deny 仍适用；参数冲突须核验 |
| S08 | [Qoder ACP](https://docs.qoder.com/cli/acp) | `qoder --acp`、自身认证、平台差异 |
| S09 | [Qoder 安装](https://docs.qoder.com/cli/installation) | npm 包、命令名、来源与 Windows arm64 限制 |
| S10 | [CodeBuddy ACP](https://www.codebuddy.ai/docs/cli/acp) | `codebuddy --acp`、环境选择、客户端工具代理与内部子事件 |
| S11 | [CodeBuddy 安装](https://www.codebuddy.ai/docs/cli/quickstart) | npm 包 `@tencent-ai/codebuddy-code` 与 native 来源区别 |
| S12 | [Copilot ACP](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server) | `copilot --acp --stdio`、启动级选项、BYOK 差异 |
| S13 | [Copilot 安装](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli) | 独立 CLI 包 `@github/copilot` |
| S14 | [Cursor ACP](https://cursor.com/docs/cli/acp) | `agent acp`、阻塞请求/通知和 MCP 范围 |
| S15 | [Cursor 安装](https://cursor.com/docs/cli/installation) | Windows 原生入口为 install?win32=true；agent 身份与更新 |
| S16 | [iFlow 官方告别公告](https://vibex.iflow.cn/t/topic/4819) | 2026-04-17 服务关闭；FAQ 允许已安装 CLI 用自定义 API |

## ACP 与 OpenSpec

| 编号 | 官方页面 | 用途 |
| --- | --- | --- |
| P01 | [ACP v1 Transports](https://agentclientprotocol.com/protocol/v1/transports) | UTF-8、NDJSON stdio、stdout/stderr 分离 |
| P02 | [ACP v1 Initialization](https://agentclientprotocol.com/protocol/v1/initialization) | 协议版本、能力协商与省略能力的含义 |
| P03 | [ACP v1 Prompt Turn](https://agentclientprotocol.com/protocol/v1/prompt-turn) | turn 结束、stopReason、权限交互与取消 |
| P04 | [OpenSpec 官方仓库](https://github.com/Fission-AI/OpenSpec) | spec-driven 工件和官方校验工具；以目标仓库安装版本为准 |

## 取舍和不确定性

本包的模块拆分、resource budget、默认 rollout 顺序与验收门槛是为 VaneHub 提出的设计，不是上游产品承诺。官方入口存在不意味着安装包所有版本可用。实现者须记录所选版本、平台、--help 和握手证据，并对源资料冲突作出明确处理。未在本包列出的认证 flag、最低 CLI 版本或内部 JSON 扩展 schema 不得凭记忆编造。
