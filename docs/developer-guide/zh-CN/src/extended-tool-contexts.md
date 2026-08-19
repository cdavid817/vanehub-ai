# 扩展工具上下文

OnePiece 的固定原生工具（shell、file、remember）之外，还有四个上下文各自提供一类**高风险能力**：跑代码、开浏览器、上公网、存产物。

| Context | 能力 | 门控 | 默认 |
| --- | --- | --- | --- |
| `code_execution` | 沙箱化代码运行时 | `VANEHUB_ONEPIECE_CODE_EXECUTION_ENABLED` | 禁用 |
| `browser_automation` | 受托管的 Playwright 浏览器 | `VANEHUB_ONEPIECE_BROWSER_ENABLED` | 禁用 |
| `web_research` | 搜索与受守卫的抓取 | `VANEHUB_ONEPIECE_WEB_ENABLED` | 禁用 |
| `artifacts` | 内容寻址的产物存储 | `VANEHUB_ONEPIECE_ARTIFACT_*` | 读取启用，发布/下载禁用 |

**只有 OnePiece 能发现或调用这些工具**。自定义 API Agent 和 CLI 封装型 Agent 无法通过复制显示名、provider 元数据或能力标签取得——桌面侧策略是权威，React 可见性仅供参考。门控清单与回退触发条件见 [OnePiece 内置工具](onepiece-builtin-tools.md)。

## code_execution：七项能力缺一不可

沙箱不是「尽量隔离」，`SandboxBackendCapabilities::ready()` 要求**七项全部成立**才算就绪：

| 能力 | 含义 |
| --- | --- |
| `restricted_identity` | 以受限身份运行 |
| `job_cpu_limit` | CPU 配额 |
| `job_memory_limit` | 内存配额 |
| `job_process_limit` | 进程数配额 |
| `kill_process_tree` | 能整棵杀掉进程树 |
| `acl_confinement` | ACL 限制文件访问 |
| `network_denied` | 断网 |

```rust,ignore
pub(crate) const fn ready(self) -> bool {
    self.restricted_identity && self.job_cpu_limit && self.job_memory_limit
        && self.job_process_limit && self.kill_process_tree
        && self.acl_confinement && self.network_denied
}
```

**这个全称合取是刻意的**。少任何一项，隔离就有缺口——「限了 CPU 但没断网」和「完全没隔离」在安全上是同一档。缺项时后端报 `IsolationUnavailable`，能力**不降级运行，而是不可用**。

### 执行状态区分「失败」与「越界」

`CodeExecutionStatus` 七态：

| 状态 | 含义 |
| --- | --- |
| `Succeeded` / `Failed` / `Cancelled` | 正常结局 |
| `TimedOut` | 超时 |
| `LimitExceeded` | 触到配额，`limit_reason` 说明是哪一项 |
| **`SandboxViolation`** | **代码试图突破隔离** |
| **`CleanupFailed`** | **跑完了但没清理干净** |

后两个必须与 `Failed` 分开：`Failed` 是代码本身没跑通，`SandboxViolation` 是安全事件，`CleanupFailed` 意味着宿主上可能残留了东西——三者的处理方式完全不同。

结果里的 `stdout_truncated` / `stderr_truncated` 是显式布尔，**不靠读者从长度猜**是否被截断。源码上限 `MAX_SOURCE_BYTES = 128 KB`。

## web_research：URL 准入 fail-closed

抓取之前，`GuardedUrlPolicy::resolve_public` 先把 URL 解析成具体地址再判定。八种拒绝原因：

| `GuardedUrlPolicyError` | 拦的是什么 |
| --- | --- |
| `InvalidUrl` | URL 本身不合法 |
| `DisallowedScheme` | 非 http/https |
| `CredentialsDisallowed` | URL 里内嵌了用户名密码 |
| `HostRequired` | 没有主机名 |
| `PortDisallowed` | 端口不被允许 |
| `ResolutionFailed` | DNS 解析失败 |
| `AddressDisallowed` | 解析到私有、回环、元数据或文档地址 |
| **`DnsRebinding`** | **同一主机的多条 DNS 答案里混有内网地址** |

**`DnsRebinding` 是这里最容易被忽略的一条**。攻击者可以让一个域名同时解析到一个公网地址和一个内网地址，抓取时按公网地址过检查、实际连接却命中内网。仓库里有一条测试叫 `private_metadata_documentation_and_mixed_dns_answers_fail_closed`——**混合答案一律拒绝**，不去赌连接时会命中哪一个。

**准入是在解析之后而不是之前**：只看字符串没法知道 `internal.example.com` 指向哪里。

## browser_automation：sidecar 与交接

浏览器跑在独立的 sidecar 进程里，上下文拥有协议、会话与动作策略、操作生命周期，以及**产物交接**——浏览器产出的截图、PDF 等不由它自己保管，而是移交给 `artifacts`。

回退触发条件写得很具体：sidecar 孤儿、策略绕过、profile 泄露、交接归属失败。**「sidecar 孤儿」单列**，因为浏览器进程活得比宿主久是这类集成最典型的故障。

## artifacts：内容寻址

`artifacts` 拥有内容寻址的 blob：媒体类型与体积校验、去重、存储容量策略。

内容寻址意味着**同一份内容只存一份**——多次执行产出相同结果时不会重复占用空间，同时 `content_hash` 让「这个产物有没有被改过」成为可验证的问题。

`CodeOutputArtifact` 是它与 `code_execution` 的接口：`artifact_id`、`content_hash`、`relative_name`、`size_bytes`、`media_type`。执行产出的文件不直接暴露宿主路径，只给逻辑标识。

三个门控把读、发布、下载分开：

- **读取**默认启用——列表、元数据、限界读取与审查。
- **发布**（`ARTIFACT_PUBLISH`）需要一次性确认与哈希绑定。
- **下载**（`ARTIFACT_DOWNLOAD`）需要哈希校验、自有保存路径、大小限制与活动内容处理。

**下载单独设门是因为它跨出了应用边界**：把内容写到用户自己选的路径上，风险与在应用内读一份 blob 完全不同。

## 共同的设计取向

四个上下文的具体机制不同，但取向一致：

- **默认禁用**，且每个门控独立——回退一个域不影响其余。
- **不完整的隔离等于没有隔离**（沙箱七项全称合取、DNS 混合答案一律拒绝）。
- **越界与失败分开报**（`SandboxViolation` vs `Failed`、`CleanupFailed` 单列）。
- **不向模型暴露宿主路径**，一律用逻辑标识。

## 与其他上下文的关系

- 工具如何进入 OnePiece 的目录、如何被调度，见 [Tool registry 与执行](tool-registry.md)与 [OnePiece native Agent](onepiece-native-agent.md)。
- 委派给外部 CLI 的隔离执行是另一条路径，见 [CLI 委派与 ChangeSet 管线](cli-delegation.md)。
- 门控、依赖与提升/回退标准见 [OnePiece 内置工具](onepiece-builtin-tools.md)。

## 设计所在

本章用于为贡献者定向，权威需求位于 `openspec/specs` 下对应能力的主规范中。
