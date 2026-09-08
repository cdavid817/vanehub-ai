# 托管 CLI 对话的 ACP 运行时

目录中十二个 CLI 里有六个不再以「每轮一个 headless 进程」运行统一对话,而是通过 **Agent Client Protocol(ACP)**:Qwen Code、Kimi Code CLI、Qoder CLI、CodeBuddy Code、GitHub Copilot CLI 与 Cursor Agent CLI。第七个新增条目 iFlow CLI 是历史兼容项,只有原生终端。本章说明这条传输链路、它如何接入既有 provider 运行时,以及每条安全边界在哪里落实。规范性要求以 OpenSpec change `extend-cli-providers-with-acp` 为准。

## 三种传输,一个网关

`ProviderTransport` 有三个值:`Terminal`(用户直接输入的 PTY)、`Headless`(每轮一个进程,解析其 stdout)与 `AcpStdio`(长驻的 agent 进程,走 JSON-RPC)。provider 在定义里声明自己支持哪些,同一事实在 tooling 目录里以 `CliManagedTransport` 镜像一份,CLI 管理页因此无需触碰运行时就能说明。一条一致性测试断言两个目录一致。

`CompositeProcessGateway` 按解析出的 provider 传输方式路由生成请求:原有五个仍走 headless 适配器,六个 ACP provider 进入 `AcpAgentProcessAdapter`,两者都不支持的 provider 在任何进程启动前即被拒绝。应用层没有任何地方按 provider id 分支。

## 线路层

ACP 模块是自研 Rust(`contexts/agent_runtime/infrastructure/providers/acp/`);评估过官方 `agent-client-protocol` crate 但未采用,因为运行时的端口是基于线程的 `Send + Sync` trait,而下表的预算需要在结构上强制。由内到外:

| 模块 | 职责 |
| --- | --- |
| `framing` | UTF-8 换行分隔的 JSON。单帧上限 1 MiB、嵌套 64 层;跨读取保留未完成的尾行,任意分片都安全 |
| `jsonrpc` | 把文档分类为 request、notification、response 或 error;其他内容(banner、裸字符串)是协议失败,绝不忽略 |
| `connection` | 拥有一个子进程、一条独立 reader 线程和唯一 writer。请求按 id 关联;未知或已超时 id 的响应被丢弃。入站队列 1,024 帧,待决请求最多 128 个,stderr 采集到有界、脱敏的 64 KiB 尾部 |
| `session` | `initialize`(协议版本 1、client capabilities)、`session/new`、对端声明 `loadSession` 时的 `session/load`,以及 prompt turn 驱动器 |
| `handlers` | 裁决 agent 发起的请求:`session/request_permission`、`fs/*`、`terminal/*`,以及 Cursor 的 `cursor/ask_question` / `cursor/create_plan` |
| `interactions` | 待决交互存储:所有等人处理的事项 |
| `binding` | 持久化的执行绑定(迁移 `cli-execution-bindings`) |
| `adapter` | `AgentProcessGateway` 实现与按绑定的连接池 |

应用日志经统一日志端口写出,绝不写入子进程的 stdout,那是协议流。

## 一个 prompt turn

一个 turn 就是一次 `session/prompt` 请求。在其响应未返回期间,驱动器持续消费入站队列,因此 agent 在 turn 中途发出的权限请求会被裁决或转交给人,并得到应答,双方都不会互相等待。这正是协议要求的递归,也是测试防范的死锁。

turn 以 prompt 响应中的 `stopReason` 结束,而非进程退出:`end_turn` 正常完成;我们自己发出 `session/cancel` 之后的 `cancelled` 是取消;`max_tokens`、`max_turn_requests` 或 `refusal` 会完成,但伴随一张可见卡片说明 agent 提前停止,且这不是经验证的任务成功。健康的连接归还给绑定供下一轮使用;一个绑定同时最多一个活动 turn。

用量与费用在这条传输上按声明为**不可用**。运行时上报 `None`,界面显示「不可用」,绝不显示零。

## 取消

取消发送 `session/cancel`,并最多等待两秒以收到 `stopReason: cancelled` 的 prompt 响应。忽略取消的 agent 会被终止其**自有**进程树;树之外的任何东西都不被触碰,它创建的代理终端随之回收。应用退出在五秒期限内关闭所有池中连接。

## 权限与代理

agent 的 `session/request_permission` 首先按会话的有效权限策略评估。`Deny` 以拒绝选项应答,工具从不执行;`Allow` 以最窄的允许选项应答;`Ask` 成为待决交互,显示为审批卡片。每个待决交互都限定在会话、turn、连接 epoch、JSON-RPC id、tool call 与策略修订之内,且恰好消费一次。关闭卡片、超时(30 分钟)、断线或取消都是**拒绝**;没有任何路径静默批准。选择「允许一次」以 agent 给出的 once 选项应答,绝不扩大为「总是」。

`fs/read_text_file` 与 `fs/write_text_file` 之所以被声明,是因为已完整实现:路径规范化后必须在跟随 symlink 之后仍落在会话授权根目录内,读取上限 8 MiB,每次写入都是经权限检查的 tool call。`terminal/create|output|wait_for_exit|kill|release` 经同一受治理的进程执行器运行,每会话最多八个终端,采集输出上限 256 KiB,拒绝属于其他会话的 terminal id。

策略旗标区分传输方式。原生终端上每个模板都投影为已安装程序自己 `--help` 记载的旗标(Qwen `--approval-mode`,Kimi `--plan`/`--yolo`/`--auto`,Qoder 与 CodeBuddy `--permission-mode`,Copilot `--mode plan`/`--allow-all-tools`,Cursor `--mode plan`/`--force`,iFlow `--plan`/`--default`/`--autoEdit`/`--yolo`);该 CLI 没有对应旗标的模板(Qoder 的只读)在 spawn 前以 reason code 拒绝,而不是以更宽松的模式启动。ACP 下只把只读姿态作为旗标传入:agent 通过 `session/request_permission` 逐次向宿主询问,宽松的启动旗标会让它跳过这一问。

## 厂商扩展

Cursor 的 `cursor/ask_question` 与 `cursor/create_plan` 是阻塞请求:显示为问题与计划卡片,以符合 schema 的回复应答、拒绝或取消。其 `cursor/update_todos`、`cursor/task`、`cursor/generate_image` 是通知,投影到会话记录且绝不应答。其他未知请求收到 JSON-RPC method-not-found 错误;未知通知被忽略。CodeBuddy 的账号环境(国际、国内、iOA)是按 profile 选择的进程级环境变量,不是凭据。Copilot 的 `--acp --stdio` 服务在进程启动时固定 tools 与 reasoning 选项,因此每个绑定有自己的进程。

## 会话绑定与恢复

首轮持久化执行绑定:provider、发行形态、安装身份、传输方式、账号 profile、工作区,以及 agent 返回的精确 `sessionId`。仅当对端声明了 `loadSession` 且本会话存在绑定时才执行恢复;agent 在 `session/load` 期间发送的回放只计数并丢弃,因此不会重复存储、计费或审批。缺少绑定、安装身份不同或对端不支持 load 时,显式开启新的外部会话。对端以 JSON-RPC `-32000`(ACP 的 `auth_required`)拒绝 `session/new` 或 `session/load` 时,报告为「未登录」并提示在终端用 CLI 登录;运行时绝不自行发起登录。turn 中途断开的连接以**中断且副作用未知**结束;prompt 绝不自动重发。

本变更之前创建的会话没有绑定,继续走原有传输路径。删除会话释放其池中连接与绑定,不触碰 CLI 的全局数据。

## 设置页可以做什么

「设置 → CLI 管理」的检测只运行有界的 `--version` 探针。它绝不启动 ACP 会话、打开登录或调用模型。该页面唯一可触发的程序启动是显式的**检查连接**操作(`check_cli_connection` 命令、`ManagedConnectionControlPort::check_connection`):以 provider 的 ACP 旗标启动已解析的可执行文件,只发送 `initialize`,然后终止进程,返回脱敏的协商摘要。**登录说明**按钮在点击时经仅允许 HTTPS 的外链服务打开目录中审阅过的 `login_docs_url`。六个 ACP CLI 的认证状态报告为 `unknown`,因为它们都没有文档化的稳定只读认证探针;「未知」不等于「已登录」。

## 设计所在

- OpenSpec change:`openspec/changes/extend-cli-providers-with-acp/`(proposal、design、provider matrix、verification results)
- 运行时:`src-tauri/src/contexts/agent_runtime/infrastructure/providers/acp/`
- provider 定义:`src-tauri/src/contexts/agent_runtime/infrastructure/providers/definitions.rs`
- 目录:`src-tauri/src/contexts/tooling/cli/domain/registry.rs`
- 相关章节:[运行时边界](runtime-boundaries.md)、[CLI 生命周期](cli-lifecycle.md)、[权限模型](permission-model.md)
