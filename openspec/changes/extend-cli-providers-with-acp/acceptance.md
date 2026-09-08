# 验收与完成定义

## 1. 完成层级

本包是待实施 change，不是测试报告。执行者必须分别报告：代码实现、自动化合约验证、真实 CLI 验证、平台验证。某一层通过不自动证明其他层。

| 结果层级 | 完成要求 | 不允许的替代 |
| --- | --- | --- |
| 规范可实施 | OpenSpec 官方 strict 校验通过；与当前主规范无未解释冲突 | 只做文本格式检查 |
| 代码完成 | 六种活跃 CLI 的必需链路、iFlow 受限路径和公共 UI/运行时/安全/持久化真实存在 | 只加卡片、占位适配器或全关能力 |
| 自动化验证通过 | 新旧 Provider 合约、ACP、权限、迁移、UI/桌面 fixture、工作流门控与项目 CI 要求通过 | 全 mock 成成功、删除负例、放松 lint/coverage |
| 真实 CLI 验证 | 每个活跃 Provider 至少有授权环境的精确版本/发行形态/平台 smoke 证据 | 仅引用官方文档或 fake suite |
| 平台认证 | 分别记录 Windows/macOS/Linux 的本平台证据与架构限制 | 一台机器外推三平台通过 |

遇到缺凭据、CLI 未安装、系统依赖不可用，可交付实际已完成代码和合约测试，但总体结论必须保留未完成的 live/platform gate。不得将 NOT RUN/BLOCKED 写为 PASSED，也不得擅自安装全局 CLI、登录用户账号或调用付费模型来消除阻塞。

## 2. 七个 Provider 的发布门槛

| Provider | 当前 change 的必需交付 | 可按能力不支持 | 不可接受 |
| --- | --- | --- | --- |
| Qwen Code | 身份/来源、终端、ACP、两轮文本、工具事件、适用审批、取消、错误、绑定和工作流门控 | 特定版本的 resume/usage/媒体 | 借用 Gemini 全部 grammar 或仅 UI 支持 |
| Kimi Code CLI | 同上，加新旧发行形态区分与 print 权限负例 | 未验证的旧版恢复、某通道特性 | 自动迁移凭据、把 -p 当人工审批路径 |
| Qoder CLI | 同上，加当前命令身份、登录 profile、平台限制 | 当前版本未声明的能力 | 历史别名直接视为新版本 |
| CodeBuddy Code | 同上，加国内/国际/iOA 环境与客户端代理权限 | 未支持的内部 Team 扩展 | 未实现就声明 fs/terminal、把内部成员变平台 seats |
| Copilot CLI | 同上，加独立 CLI 与进程级参数隔离 | 不满足 profile 的认证/模型功能 | 使用 gh 扩展替代、跨策略共用服务进程 |
| Cursor Agent CLI | 同上，加 agent 身份、问题与计划阻塞请求 | 未支持的扩展、MCP 范围 | 收到阻塞请求一直转圈或自动批准 |
| iFlow CLI | legacy 可见性、真实状态说明、本地 opt-in 终端、策略检查、禁止自动化 | ACP、受管聊天和自动化明确不支持 | 宣称官方服务仍在、默认安装或自动迁移 |

对无法满足硬性策略的 PTY，必须拒绝或要求用户明确修改要求，不能以“终端原生行为”绕过安全边界。可选能力不支持必须具备可观察的 reasonCode 和对应负向测试。

## 3. 跨功能硬性标准

**旧功能兼容**：原有五种 Provider 的稳定 ID、默认选择、合法启动参数、权限、历史会话与统计没有退化。

**协议与资源**：一轮结束不依赖进程退出；发送 prompt 后保留 stdin； reader 不因等待 prompt 响应阻塞审批；有明确预算、取消回收与最终状态唯一性。

**权限闭环**：拒绝后操作没有执行；一次批准不扩权；旧 epoch/错 seat/重复响应无效；关闭、超时、断线不批准；未声明的工具代理不可调用。

**身份与数据**：不会把别的 `agent` 程序当 Cursor；两个 `kimi` 发行形态可区分；精确会话恢复、回放不重复记账；中断不盲重放；迁移保留旧记录。

**UI 与自动化**：安装/认证/兼容/能力分开；缺 token 不显示零；每个 seat 与任务独立；自动任务遇到人机交互有明确终态/干预路径。

**测试真实性**：synthetic fixtures、真实 CLI、不同平台证据分开；测试不访问真实用户数据库、凭据或默认模型端点。自有临时进程必须回收，不能留下守护进程。

## 4. 需求到任务的覆盖

| Capability | 主要任务 | 验证层 |
| --- | --- | --- |
| provider-plugin-sdk | 1.*, 3.*, 13.* | Rust domain / conformance / V1 回归 |
| cli-environment-management | 2.*, 5.*–9.* | discovery、source plan、auth、desktop fixture |
| expanded-cli-provider-adapters | 5.*–9.*, 13.* | 逐家合约和真实 smoke |
| acp-agent-runtime | 3.*, 10.*, 13.* | 协议子进程、状态、取消、背压 |
| acp-permission-bridge | 4.*, 8.4, 11.4, 13.* | 策略、交互所有权、文件/终端负例 |
| cli-session-transport-binding | 10.*, 13.* | SQLite migration、resume/replay、资源归属 |
| cli-provider-capability-ui | 11.*, 13.* | Vitest、Playwright、Web/mock 和桌面 |
| cli-provider-automation | 12.*, 13.* | 多 seat 隔离、调度干预、幂等恢复 |

逐项 requirement/scenario 索引见 [requirements-index.json](verification/requirements-index.json)。执行完成后应在结果记录中写入实际测试文件与命令，不以本表作为测试已经存在的证据。

## 5. 本次不自动做的操作

不 commit、push、开 PR、切分支、归档 OpenSpec、更改用户 CLI 全局配置、删除用户数据或 Worktree。用户另行授权后再按项目治理流程执行。OpenSpec 未归档前仅在 change 中存放 delta specs，不提前用新规范覆盖主规范或编辑 archive。
