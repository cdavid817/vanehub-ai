# OnePiece 内置工具操作指南

OnePiece 是唯一被允许发现或调用扩展原生工具集的 Agent。自定义 API Agent 和 CLI 封装型 Agent 无法通过复制显示名称、provider 元数据或能力标签来获得这些能力。桌面侧策略是权威来源;React 可见性仅供参考。

## 发布门控

在启动 VaneHub 之前,把门控设置为精确值 `1`。缺失值、`0`、`true` 以及其他字符串都会保持禁用状态。各门控相互独立,因此回退某一个域不会禁用既有的文件、搜索、shell、Skill、LSP、MCP,或其他与 OnePiece 无关的行为。

| 环境变量 | 能力 | 默认 | 提升标准 | 回退触发条件 |
| --- | --- | --- | --- | --- |
| `VANEHUB_ONEPIECE_ARTIFACT_READ_ENABLED` | Artifact 列表、元数据、限界读取与审查 | 启用 | 数据库迁移、完整性、保留策略与预览测试通过 | 完整性不匹配、路径泄露或无界预览 |
| `VANEHUB_ONEPIECE_BROWSER_ENABLED` | 受托管的 Playwright Browser 读取/副作用与交接 | 禁用 | Sidecar 兼容性、导航策略、清理与交接 E2E 通过 | Sidecar 孤儿、策略绕过、profile 泄露或交接归属失败 |
| `VANEHUB_ONEPIECE_WEB_ENABLED` | DuckDuckGo 搜索与受守卫的抓取 | 禁用 | Provider fixture、SSRF/重定向、扩展上限与来源元数据测试通过 | Provider 漂移、地址策略绕过、凭证继承或无界响应 |
| `VANEHUB_ONEPIECE_CODE_EXECUTION_ENABLED` | 独立的代码沙箱 | 禁用 | 平台隔离、离线网络、进程树、配额与清理测试通过 | 隔离见证丢失、宿主文件访问、网络访问或孤儿进程 |
| `VANEHUB_ONEPIECE_OCR_ENABLED` | 本地 PaddleOCR 提取 | 禁用 | 受托管的 worker/PDFium 校验和、兼容性、限制与隐私测试通过 | 远程回退、协议漂移、校验和失败或私密内容日志泄露 |
| `VANEHUB_ONEPIECE_ARTIFACT_PUBLISH_ENABLED` | 经过认证的 Artifact 发布 | 禁用 | 一次性确认、哈希绑定、过期与访问控制测试通过 | 哈希不匹配、确认绕过或可见性/过期失败 |
| `VANEHUB_ONEPIECE_ARTIFACT_DOWNLOAD_ENABLED` | 受控的桌面下载 | 禁用 | 哈希校验、自有保存路径、大小限制与活动内容处理通过 | 源路径暴露、覆盖、哈希不匹配或不安全的文件激活 |
| `VANEHUB_ONEPIECE_DELEGATION_ANALYZE_ENABLED` | Claude Code/Codex CLI 分析 | 禁用 | 被动就绪、协议 fixture、脱敏、配额与清理通过 | 凭证/转录泄露、协议漂移、重试循环或子进程逃逸 |
| `VANEHUB_ONEPIECE_DELEGATION_EDIT_ENABLED` | 隔离的委派编辑与 ChangeSet 封存 | 禁用 | 分析标准加上独立工作区、离线子命令与完整 ChangeSet 校验通过 | 目标变更、证据不完整、未封存输出或隔离丢失 |
| `VANEHUB_ONEPIECE_DELEGATION_APPLY_ENABLED` | 精确的一次性 ChangeSet 应用 | 禁用 | 干净基线预检、独占租约、回滚胶囊、精确校验、崩溃恢复与重放测试通过 | 部分应用、过期审批、锁丢失、回滚不确定或恢复回归 |

回退指的是仅移除受影响的环境变量并重启桌面运行时。追加式的数据库迁移和已保留的证据不会被删除。在途的自有工作会被取消并回收;一次需要恢复的应用仍然可见,供人工检视。

## 依赖与就绪

- 运行 `npm install` 安装固定版本的 Playwright 包,然后用 `npx playwright install chromium` 准备其受托管的 Chromium。原生 sidecar 使用隔离的临时上下文,绝不导入用户的常规浏览器 profile。
- 通过「设置 → 扩展」安装并启用 PaddleOCR。OCR 就绪需要一个受托管的 PaddleOCR 3.x 推理协议,以及校验和经过验证的 PDFium 渲染器。不要把二进制文件手动放进应用数据目录。
- 通过厂商支持的安装器安装 Claude Code 或 Codex CLI,并让可执行文件通过常规进程环境可被发现。就绪检查使用被动的版本/帮助/认证检查,不消耗模型配额。
- 沙箱只接受经过审查的 Python 3.11–3.14 与 Node.js 20–24 运行时。它不安装包,也绝不回退到普通 shell 执行。

OnePiece 配置页分别展示每个能力与模式。稳定的理由包括 `disabled`、`backend_unavailable`、`policy_unavailable`、版本/依赖失败,以及隔离失败。就绪检查不会打开浏览器、运行用户代码、对内容做 OCR,或启动外部 AI 任务。

## 权限与数据边界

任意代码执行、有副作用的 Browser 操作、已保留的下载、委派启动、Artifact 发布与 ChangeSet 应用都需要经过统一权限评估。ChangeSet 应用始终使用一次不可记住的、一次性的审批,并绑定到 Artifact id/内容哈希、diff 哈希、仓库身份、精确基础 commit,以及干净状态见证。

Artifact 是由内容寻址 blob 支撑的不可变逻辑记录。工具之间的二进制传递使用 Artifact id,而非任意路径。发布只会通过 VaneHub 经过认证的边界暴露一个 Artifact;它不会创建一个公共 Internet URL。保留策略会保留被引用的 Artifact,并通过受治理的清理路径移除过期、未被引用的 blob。

持久化日志只包含限界标识符、结果码、哈希、计数与计时。凭证、授权头、提示词、页面/文件正文、OCR 文本、隐藏推理、provider 转录、原始子进程输出,以及私密路径,都会在统一日志服务持久化一个事件之前被移除。

## 恢复与故障排查

对于 Browser 故障,确认门控、Node/Playwright 安装、受托管 Chromium 的可用性,以及 sidecar 协议版本。崩溃的 sidecar 最多重启一次;反复失败即为终态。

对于 Web 故障,区分 `provider_protocol_changed` 与 URL 策略拒绝。私有、回环、链路本地、元数据、携带凭证,以及非 HTTP(S) 目标在每次重定向时都被有意拦截。

对于沙箱或 OCR 故障,使用就绪理由,而不是绕过隔离或校验和。缺失证据会使该能力不可用;不存在 shell 或远程 OCR 回退。

对于委派,校验经过审查的 CLI 版本、必需的标志、认证、目标工作区身份,以及编辑隔离支持。在宿主证据一致之前,provider 的声明仅供参考。失败或被取消的尝试不会自动重试。

对于 ChangeSet 应用,当 UI 报告需要人工恢复时,停止自动变更。检视已保留的恢复引用和安全说明。绝不要为了继续一次自动化尝试而执行 stash、reset、merge、rebase、cherry-pick、commit、push、解决冲突,或部分应用文件。

## Web/mock 运行时

Web/mock 实现了相同的 TypeScript 服务契约,其确定性记录被标记为 `simulated`。原生副作用返回 `desktop_runtime_required`;mock 不会声称发生过浏览器、网络抓取、沙箱、OCR worker、本地发布、外部 CLI,或仓库变更。
