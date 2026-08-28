# 飞书真实环境资格验证

本手册仅用于在专用飞书测试租户上执行显式启用的桌面端资格验证。它不适用于确定性 fixture 测试，也不得复用个人或生产应用。

## 最小权限应用配置

创建企业自建应用并开启机器人能力。VaneHub 的单聊文本接收与回复只授予以下应用权限：

| 用途 | 飞书权限 |
| --- | --- |
| 接收用户发给机器人的单聊消息 | `im:message.p2p_msg:readonly` |
| 以应用机器人身份发送回复 | `im:message:send_as_bot` |

存在以上两个窄权限时，不要授予范围更大的 `im:message`。本次验证不需要群消息、通讯录、用户 ID、附件、卡片、表情回应或 user-agent 权限。VaneHub 使用消息事件已经携带的 `open_id` 与 `chat_id`，不需要 `contact:user.employee_id:readonly`。

飞书官方[接收消息事件文档](https://open.feishu.cn/document/server-docs/im-v1/message/events/receive)列出了只读单聊权限与事件类型；官方[发送消息 API 文档](https://open.feishu.cn/document/server-docs/im-v1/message/create)列出了 `im:message:send_as_bot`，并要求应用开启机器人能力且接收者位于机器人可用范围内。

## 操作员配置

1. 在飞书开发者后台创建或打开专用测试租户的企业自建应用，并开启**机器人**能力。
2. 在**权限管理**中添加上述两个权限，移除资格验证不需要的其他权限。
3. 在**事件与回调**中选择**使用长连接接收事件**。VaneHub 使用此模式，不要配置 Webhook 请求地址。
4. 在**消息与群组**分类添加**接收消息 v2.0**，确认事件标识为 `im.message.receive_v1`，订阅身份为应用身份。
5. 创建并发布应用版本。权限、事件、机器人能力与可用范围变更在版本于测试租户生效前不算配置完成。
6. 把应用可用范围限制为最小的专用测试用户集合；若只有一名操作员，就只加入该操作员。
7. 在飞书中打开与应用机器人的一对一聊天并发送一条无敏感内容的准备消息。通过获授权的诊断路径取得该 p2p 事件的 `event.message.chat_id`；它属于外部标识，不得提交到仓库或写入保留证据。
8. 在 VaneHub 桌面端设置中配置默认 Agent 与项目，通过正常的只写设置流程保存飞书 App ID/App Secret 并启用连接器；然后在目标会话的信息面板打开 IM 并绑定专用聊天。

飞书官方[事件订阅概述](https://open.feishu.cn/document/server-docs/event-subscription-guide/overview)说明了长连接模式以及添加事件后发布应用的要求。其重试说明也表明重复投递是正常情况，资格验证必须覆盖该场景。

## 预检与执行

在仓库 worktree 中打开全新的 PowerShell 会话。所有值仅在运行时提供，不得写入文件、命令参数、截图、issue 或提交到仓库的脚本。

```powershell
$env:VANEHUB_FEISHU_LIVE_QUALIFICATION = "1"
$env:VANEHUB_FEISHU_TEST_TENANT = Read-Host "专用测试租户" -MaskInput
$env:VANEHUB_FEISHU_APP_ID = Read-Host "飞书 App ID" -MaskInput
$env:VANEHUB_FEISHU_APP_SECRET = Read-Host "飞书 App Secret" -MaskInput
$env:VANEHUB_FEISHU_PERMISSIONS_CONFIRMED = "1"
$env:VANEHUB_FEISHU_LONG_CONNECTION_CONFIRMED = "1"
$env:VANEHUB_FEISHU_TEST_CHAT_ID = Read-Host "专用 p2p chat_id" -MaskInput
$env:VANEHUB_FEISHU_LIVE_OPERATOR = "1"
npm run test:desktop:feishu-live
```

设置 `VANEHUB_FEISHU_LIVE_OPERATOR=1` 后，终端会逐步输出一次性配对码和待发送的无敏感测试文本。每一步最长等待 10 分钟；不要提前发送下一条消息。未设置该变量时只运行凭据、鉴权、连接生命周期和无效凭据阶段，真人入站矩阵继续报告 `NOT RUN`。

飞书平台重试使用同一稳定事件 ID，普通地重复发送相同文本会产生新的事件，不能作为去重证据。只有本次真实长连接实际观察到平台重投时，该场景才能记为 `PASSED`；否则单独记为 `BLOCKED`，确定性 fixture 的去重结果不能替代它。

未显式启用时入口报告 `NOT RUN`，缺少任一前置条件时报告 `BLOCKED`。真实环境结果与 fixture 证据相互独立。保留的真实环境工件只能包含安全状态码与时间戳，严禁包含凭据、租户标识、聊天标识、提示词、回复内容或原始协议载荷。

运行后关闭 PowerShell 会话，或移除全部八个 `VANEHUB_FEISHU_*` 变量。桌面运行器也会在清理阶段删除本次运行拥有的凭据引用。如果清理结果不是 `CLEARED`，应把本次运行视为失败，先从 VaneHub 删除该专用应用凭据再重试。

## 资格验证清单

每个真实场景分别记录为 `PASSED`、`FAILED`、`BLOCKED` 或 `NOT RUN`：

- 鉴权与长连接生命周期；
- 单聊文本接收与重复投递；
- 单 Agent 最终回复；
- 多 Agent 指定席位、默认席位与无效席位路由；
- Unicode 安全的回复分片；
- 会话禁用、重新启用与桌面端重启；
- 无效凭据拒绝与恢复。

fixture 成功不能替代真实结果。若任何工件包含凭据、外部标识、消息内容或原始事件载荷，立即停止验证。
