# IM connector

native 侧负责 IM connector 的配置、凭证、路由与入站投递。远程工作空间与 IM 工作流在用户指南中介绍;本章介绍 native 侧设计。

## 五个内置 connector

五个可独立配置的内置 connector,具有稳定 id:`feishu`、`telegram`、`dingtalk`、`wecom` 与 `weixin`。connector 描述符列表返回全部五个 connector,并附带本地化的展示元数据、配置字段、能力以及实验性标志。个人微信(`weixin`)被标记为实验性;其余四个不是。

## 第一版本的直接消息范围

每个 connector 仅接受文本**直接消息(direct message)**。群组消息与非文本内容在第一版本中被排除在 Agent 执行之外:一条群组消息会被确认或消费,但不会创建 VaneHub 消息或 Agent 生成。一条有效的文本直接消息会从其平台事件中归一化,并提交至共享入站路由器。

## 消息流转与路由

入站消息从平台事件到 Agent 执行的完整链路如下。首版本仅处理文本直接消息；群组消息与非文本内容只确认回执，不创建会话。

```mermaid
sequenceDiagram
    autonumber
    participant Platform as IM 平台
    participant Connector as Connector 适配器
    participant Router as 共享入站路由器
    participant Agent as 目标 Agent
    participant Workspace as 目标工作区
    participant Session as 会话执行
    participant Out as 出站回推

    Platform->>Connector: 平台事件(webhook/poll)
    alt 文本直接消息
        Connector->>Connector: 归一化为文本直接消息
        Connector->>Router: 提交归一化消息
        Router->>Router: 投递准入检查(pending_delivery_admission)
        Router->>Agent: 路由到目标 Agent + 工作区
        Agent->>Workspace: 绑定工作区
        Workspace->>Session: 创建/复用会话执行
        Session->>Out: 生成回复
        Out->>Platform: 回推文本消息
    else 群组消息或非文本
        Connector->>Platform: 确认/消费,不创建会话
    end
```

**入站投递准入**：路由器在创建会话前会检查 `pending_delivery_admission`。每个会话(chat)的挂起投递上限为 `MAX_PENDING_PER_CHAT=8`，超过后返回 Busy 但**不阻塞**平台事件确认。运行时还维护两个全局水位：总 pending work 上限 64，active Agent generation 上限 8。空闲 lane 会被回收，以让新的路由请求可以重用执行槽位。

**微信授权流程**：`weixin` connector 被标记为实验性，凭据获取走 QR 授权流程。`AuthorizationStatus` 有六个状态:`Waiting` → `Scanned` → `Confirmed`,可分别转至 `Expired`/`Error`/`Cancelled`;凭据写入平台钥匙串(keychain)，读取走 zeroizing reads，使用后立即清零内存副本。

**出站策略**：首版本仅支持把 Agent 生成的文本结果直发回原会话。入站路由依赖预先保存的默认路由配置——启用 connector 前必须先配置默认路由(目标 Agent + 工作区)，否则归一化后的消息无法落地执行。

## 关键常量与凭据

入站投递的并发与水位控制由 `communications/domain/delivery.rs` 与 `infrastructure/runtime_manager.rs` 共同承载:

- **`MAX_PENDING_PER_CHAT = 8`** —— 单个会话(chat)的最大挂起投递数。超过后返回 `Busy`,但**不阻塞**平台事件确认。
- **总 pending work 上限 64** —— 跨所有会话的待处理工作总量上限,触顶后新路由请求等待空闲 lane。
- **active Agent generation 上限 8** —— 同时运行的 Agent 生成数上限;空闲 lane 会被回收,以让新的路由请求重用执行槽位。
- **去重与检查点** —— 入站消息经 `dedup` 去重、`checkpoint` 记录投递进度,保证幂等;调度去重按批次(每批至多 512 行)保留。
- **微信安全上下文** —— 每个 chat 的微信安全上下文元数据有上限的有界保留,restart/stale-refresh/rollback 都有覆盖;`clear` 在移除每个被追踪的 per-chat 安全上下文前先停止运行时。

凭据由 `communications/infrastructure/credential_adapter.rs` 经平台 keyring 边界保管:

- **zeroizing reads** —— 凭据从 keyring 读出后立即复制到 zeroizing 缓冲,使用后立即清零内存副本。
- **稳定 account references** —— 凭据以稳定 account 引用关联,不随连接器重命名而失效。
- **微信授权迁移** —— 旧版微信凭据有迁移/删除路径,授权失败返回安全错误而非裸露凭据。

## 设计所在

本章用于为贡献者定向。权威需求位于 spec 中。

- [openspec/specs/im-connector-management](../../../../openspec/specs/im-connector-management/spec.md)

IM connector 位于 `communications` 限界上下文;参见 [Native bounded contexts](native-contexts.md)。
