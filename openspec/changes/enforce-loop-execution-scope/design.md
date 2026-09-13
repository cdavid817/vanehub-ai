# Design

## 1. 基线与确定的问题

基线 SHA：`75f23d81406bdc1fd4d6b4d5a41fbf90e395c4fc`。以下路径均相对仓库根目录，行号只用于定位基线，实施以符号为准。

| 位置 | 现状 | 本变更的落点 |
|---|---|---|
| `src-tauri/src/contexts/agent_runtime/application/loop_service.rs`，readiness / validate_start | readiness 有范围检查，权威 start 更窄 | 同一无副作用校验器 + 启动事务复核 |
| `application/loop_worker_prompt.rs`（以下 application 路径相对 agent_runtime） | scope 是提示词文本且有 `.take(32)` | 提示词只是说明；不能作为完整策略或悄悄截断配置 |
| `application/loop_models.rs`、`loop_worker.rs`、`loop_verifier.rs` | role session 请求没有可信范围绑定 | backend 产生 scope ref，经 session gateway 传递 |
| `application/loop_orchestrator.rs`、`loop_orchestrator_decision.rs` | verification 复用主要按命令 id；决定无范围证明 | 绑定输入指纹并加入阶段门禁 |
| `infrastructure/loop_verification_process.rs` | 程序白名单和 cwd 限制，不约束脚本副作用 | 所有验证子进程接受同一准入与能力评估 |
| `application/loop_control.rs`、`domain/loop_engineering.rs` | accept 直接成功；resume 清除暂停原因 | 接受证据封存/CAS；恢复先验证绑定和能力 |
| `infrastructure/providers/acp/handlers.rs`，`handle_read` | 只拒绝 Deny，Ask 仍可读取 | Allow / Deny / Ask 显式分支 |
| `infrastructure/providers/acp/interactions.rs`、`session.rs` | 已有 FileWrite 等延迟交互 | 新增 FileRead，接入全部交付/取消/显示分支 |
| `src-tauri/src/contexts/permissions/application/evaluation_service.rs` | 评估失败返回 Ask | 同一评估入口同时提供健康状态，禁止故障 Ask 被执行 |
| `src/loop-center/loop-preflight-dialog.tsx` | passed 细节不总展示 | 独立且常显执行覆盖程度与限制 |
| `src/contracts/loop.ts`、`src/types/loop.ts`、`src/services/loop-service.ts` | 合约缺少范围版本、能力评估和证据 | 类型化扩展，Tauri/Web 同步 |

上表缩写路径 `application/*`、`infrastructure/*`、`domain/*` 的前缀均为 `src-tauri/src/contexts/agent_runtime/`；permissions 的文件已给完整路径。

## 2. 权限语义与威胁边界

路径配置定义**本次 Loop 可以改变的工作区内容上限**。`allowedPaths` 非空；`["."]` 表示显式允许整个 worktree，但仍减去保护路径和保留控制路径。`protectedPaths` 可为空。读权限独立：保护路径不自动变成禁止读取，读取是否允许由 `file.read` 策略决定。

宿主可接收的托管操作在执行前按以下顺序处理，不改变现有稳定 Agent principal。严格模式的其他通道须由合格执行环境阻断越界；审计模式的不透明 CLI 内部动作只绑定所属执行树和已披露限制，不能宣称逐操作走过以下审批：

1. 建立可信 run/session/generation/role/Runner 归属，解析被冻结的 scope。
2. 校验角色、真实资源及范围上限。无绑定、越界或不能安全解析，直接拒绝，不生成可扩大范围的批准。
3. 将合格请求交给现有统一权限入口，继续执行模板、grant、显式 Deny、Plan Mode、Skill manifest 等规则。
4. Allow 才能交付；健康的 Ask 走现有审批；Deny 或评估故障不得产生副作用。审批交付前重复检查上限、当前策略和资源身份。

`mcp.tool` 在普通权限评估中仍保持原有 Ask floor。本变更的范围准入在该评估之外：它可以拒绝将越界调用交给执行器，但不得把原有 evaluator 的 MCP Ask 规则改成 Allow 或 Deny。Skill 的独立 manifest 语法不变，其权限上限与 Loop 上限取交集，不直接混用两套路径解析器。

严格模式的威胁模型覆盖模型生成的路径、Agent/工具输入、脚本间接写入、错误配置及普通文件系统竞态。不承诺抵抗已控制 VaneHub 原生进程/数据库的攻击者或管理员；但严格模式不能依靠模型服从提示词或不触碰未声明通道来成立。

产物审计模式的保证更窄：在合作 CLI、宿主控制面可信的前提下检测意外工作区越界和检查最终产物。未隔离的同 UID CLI 可能主动篡改宿主数据库/证据，单独换一个宿主目录或对清单做 hash 不能消除这个风险。该模式的确认与持续展示必须明确这一信任前提，不能声称抵抗主动篡改；若需要该保证，应使用具备独立权限隔离的执行/证据边界并重新证明能力，不能把审计模式升级标记为严格模式。

## 3. 路径语法及真实资源检查

### 3.1 配置语法 v1

- 配置是字面相对路径，按路径组件匹配“该路径及其后代”。`src` 不匹配 `src-old`。`src` 与保护项 `src/generated` 合法，禁止整个 `src` 则该允许项无可用范围；若所有允许项均被覆盖则拒绝。
- 明确采用 `/` 作为存储分隔符；去除冗余 `.` 组件、重复分隔符和尾分隔符，稳定去重排序。空白分隔/去空白只属于 UI 输入层，不静默改变实际文件名中的空格。
- 拒绝 `..`、绝对路径、drive/UNC/device/ADS 路径、NUL/控制字符及 glob 元字符。Windows 输入的 `\` 可作为分隔符规范化；Unix 的字面反斜线文件名无法由 v1 表达，明确拒绝而不是重新指向其他文件。
- 大小写等价由实际目标卷决定，不能全平台转小写或擅自做 Unicode 折叠。不能确定卷语义时 fail closed。
- 配置条目/字节数上限由版本化校验常量统一约束；超限报告错误，绝不能截断后声称完整生效。提示词可以有展示摘要，但必须带省略提示和完整 scope digest；执行器始终使用完整绑定。
- `.git` 文件/目录、实际 Git common-dir、原生策略和证据存储是保留控制资源。用户不得用 `.` 或 grant 开放它们。worktree 建立/清理的宿主操作使用窄化的内部能力，仅限指定仓库控制路径，不能继承给 Agent。

### 3.2 副作用检查

运行根是实际新 worktree 的目录身份，不能回退到原仓库、会话 cwd 或前端传入路径。检查真实目标及其祖先：写已有文件检查目标，创建检查最近存在的父目录；rename 同时检查源与目的，递归删除检查受影响子树，模式/类型/链接变更也是变更。

在同一个安全执行边界内使用 handle-relative/no-follow 操作或已验证的等价系统能力。仅 `canonicalize(path)` 后按字符串重新 open 存在 TOCTOU，不符合要求。链接/junction/reparse point 不能证明安全时拒绝；禁止借链接走出根或进入保护资源。hardlink 不允许原地写共享 inode，除非证明全部别名均安全；安全的本地原子替换可以作为另一实现，但不能修改范围外别名。

为 Windows、macOS、Linux 分别登记可证明的保证；某平台未实现可靠检查就标注不支持对应能力，不能用 Web mock 或其他系统测试替代。

## 4. 能力评估与两种启动模式

“用户要求什么”与“执行器实际覆盖什么”是两个字段。每个角色、验证命令及启用的副作用通道都要返回评估：

| coverage | 意义 | 可以满足默认模式 |
|---|---|---|
| `complete-enforcement` | 所有可达变更通道均受执行前上限约束 | 是，且证明仍有效 |
| `mediated-tools-only` | 宿主托管操作受控，仍有未覆盖通道 | 否 |
| `artifact-validation-only` | 未证明预防能力，只能检查最终工作区产物 | 否 |
| `unsupported` / `unknown` | 接口不支持或证明缺失/失效 | 否 |

能力证据绑定 stable Agent/provider ID、transport、Runner/平台、可执行文件身份与版本、adapter 修订、启用工具和子进程通道及真实 containment 配置；运行时重查。支持 ACP、采用 hook、设置沙箱参数或使用 worktree 本身都不是 `complete-enforcement` 的证明。

`preventive-required` 是新定义默认模式。只要任何可达变更通道没有完整约束，权威 start 就拒绝；可建议选择已支持的 Agent/工具组合或缩小启用通道。此变更不要求新造通用 OS 沙箱，但必须交付一个完整严格 Loop：可信 API Worker 只启用已托管文件工具，关闭原始进程/CLI/MCP/未受控 Skill 通道；必需验证使用下文的原生内置检查；Verifier 只启用受控读取与结构化建议；最后通过真实原生证据门禁接受。模型输出可以用固定 provider fixture 生成，文件、worktree、持久化和门禁不得用 mock 替代。不支持的其他组合诚实阻断。

`artifact-audited` 是兼容模式。必须同时满足：定义显式选择；每次人工 start 看到确切范围和未覆盖通道并确认；确认绑定定义版本、scope digest 和能力评估；托管工具依旧执行前阻断；原生能进行完整产物检查；Verifier 仍符合只读保证。不能用老版本 UI、省略参数、调度/无人值守、自动恢复来隐式启用。未知执行身份直接拒绝；已知缺少 containment 的 CLI 可以列为仅产物检查。该模式不能称为“路径已隔离”，也不能证明未观察到的临时写入或根外修改不存在。

程序白名单只是额外约束。`python -c`、`node -e`、npm/cargo 构建脚本、shell、MCP、Skill 工具、CLI 内部工具及后代进程都可能写文件。严格模式必须限制/关闭未覆盖通道，或由合格的执行环境约束全部后代；把程序名列入白名单不够。Verifier 的所有变更通道在两种模式都必须执行前阻断，不能以 prompt 或事后 diff 替代只读。

首个严格验证正路径：给结构化 verification record 增加版本化 `kind: native-check | process`。旧记录缺 kind 按原有 `process` 解码，不能默默获得受控标记；新建严格定义默认提供 `kind=native-check`、`program=patch-whitespace`、空 args、cwd=`.` 的必需行。原生检查器在进程内对 baseline/current 快照的新增文本行检查尾随空格/制表符，不启动 Git、shell、脚本、插件或任意可执行文件，也不修改文件。它使用安全读取/只读快照和适用的现有读取准入，保留 timeout、required、退出状态/证据合约；预算不足、解码无法确定或读取失败返回 unverifiable，不伪造通过。二进制条目明确记为该空白检查不适用，但仍接受完整 scope hash/类型检查。其他 native-check 名称/args 组合拒绝；此检查只证明固定空白规则，不能冒充编译、测试或业务验收。用户仍可选择 process checks，但严格模式须有其合格执行能力，不能用 builtin 替代用户已配置的必需检查。

原有 CLI 模板更改仍只改变未来进程的启动参数。本次 Loop 的受控操作按当前策略复核；权限收紧而现有黑盒进程不能动态受控时，取消并暂停该执行，不能假装已修改其启动参数。策略放宽绝不能扩大被冻结的范围。

## 5. 数据、服务边界及执行流程

以下为实现所需的逻辑字段，名称可以遵循项目惯例调整，但语义和版本必须保留：

| 记录 | 核心内容 |
|---|---|
| Definition scope | schemaVersion、allowed/protected 字面路径、requestedMode、definitionVersion |
| Execution assessment | per-role/per-surface coverage、blockers、bounded limitations、witness digest、assessedAt、simulated |
| Immutable run scope | definition snapshot/hash、mode、确认记录、run/iteration/role ownership、scope digest、base commit、worktree identity、baseline manifest ref、capability witnesses |
| Evaluation context | stable principal + run/session/generation/epoch/role、trusted scope ref、resource identity、Runner/authority revision、evaluation health |
| Scope evidence | phase/input/command fingerprint、baseline/current manifest hashes、scope/witness digest、completion status、sticky violation refs、sealed evidence id |

范围的“不可变”表示运行上限不可扩大，**不是**冻结一个永久 Allow。策略撤销、当前 Deny、取消和过期继续有效。宿主接收的子调用继承原上限与所有其他 manifest/Plan Mode 上限的交集，丢失 provenance 时拒绝；审计模式不透明内部动作记录为未覆盖通道，其执行树归属或退出状态无法确认时不能封存阶段证据。不激活现有未开放的 principal delegation 功能。

### 5.1 保存和启动

保存时进行可重复的语法校验；readiness 做完整无副作用评估，不创建 run/worktree/session。start 不信任 readiness token：重新加载定义/Agent/当前依赖并运行同一校验器，在事务中以 definition version、active-run 唯一性/CAS 和能力修订提交 queued run 及冻结相对范围。已知阻断发生于 queued 之前。

worktree 必須异步准备，因此真实根身份不能在尚不存在时伪造。preparing 完成后确定根、base commit、完整 baseline 和角色绑定，在原生持久化成功后才启动第一个 Agent/命令。该阶段竞态或存储失败可留下已记录的失败/暂停 run，但不得产生 Agent 副作用。重试必须重新证明身份，不能借旧路径字符串继续。

`LoopRoleSessionRequest`、`RunLoopVerificationRequest` 和 gateway 增加原生授权引用；前端不传可自行签发的 token。UI → loop service → Tauri adapter → command DTO/mapper → application use case；Web adapter 同步模拟。扩展 `src/contracts/loop.ts` 后同步 `src/types/loop.ts`、表单、clone/enable 转换和所有 adapter，避免手工重建 save input 丢字段。

### 5.1.1 审计确认的最小合约

readiness 只返回事实、blockers 和 `acknowledgementRequired`，不等待确认，因此不会与确认 UI 循环依赖。`prepareLoopAdmission({action: start|resume|continue, definitionId?, runId?, expectedRevision})` 返回原生 assessment、scope/witness digest 及短期 challenge；此操作无 run/worktree/session 副作用。UI 展示具体限制与控制面信任前提后，通过 `acknowledgeLoopAudit({challengeId})` 取得原生存储的 receipt id。该 receipt 仅记录兼容模式确认，不是权限 grant，不允许越界操作，也不复用 file/tool pending approval。

receipt 绑定目标 action、definition 或 run、预期 revision、相对 scope digest、capability assessment、启动客户端上下文和生成时间，5 分钟有效且只消费一次。start/resume/continue 服务参数增加 `expectedRevision`、`idempotencyKey` 和可选 `auditAcknowledgementId`；严格模式不需要 audit receipt。原生重新检查实际配置/能力，在相同业务事务中 CAS 状态并消费 receipt；重复相同 idempotencyKey 返回已有 operation，不能二次执行。跨 action/run、超时、应用重启、时钟回退、配置或能力变化使 receipt 失效。缺参数的旧调用不能默许 audit；Web 模拟相同规则。失败且未提交时不能执行子操作；事务外发现的能力变化仍在 spawn 前拒绝，不因 receipt 已消费而绕过检查。

### 5.2 运行和接受

每次工具交付做真实资源检查；每个阶段在后代写入者退出/取消并完成回收后做完整产物扫描，跨过 Worker、验证、Verifier 边界前存储证据。保护/越界变更产生粘性违规记录；即使之后回滚，该条已观察到的违规也不能消失。

原生决策进入 awaiting-acceptance 时要求必要检查通过、Verifier 无 revise/blocked、范围证据完整且无违规。接受是有耗时 IO 的原生操作：通过现有 operation registry 提交异步 acceptance 检查，保持 UI mutation pending；不要把全树扫描塞进同步 Tauri command。可以扩展服务为 `requestLoopAcceptance({runId, expectedRunRevision, expectedScopeDigest, expectedEvidenceId}) -> OperationRef`，参数仅是并发前置条件，授权始终从后端载入。原有 accept 入口必须走同一门禁，不能保留直达 `run.accept()` 的绕行路径。

最终接受按固定顺序执行：取得 run/worktree 的宿主写入 lease → 停止并回收全部所属写入者 → 把实际待接受文件内容复制为宿主管理的不可变 artifact objects → 根据复制内容生成完整 manifest，核对已验证的输入/输出指纹与当前根/策略/能力/范围 → 以 run revision 和 evidence id 做 CAS 提交成功。必须保存实际内容与 manifest，不能仅存 hash 后声称可复核完整产物。宿主 lease 不排斥外部编辑器/进程；复制或校验中出现变化且不能建立与验证证据一致的稳定快照时返回 `scope-unverifiable`，SQLite revision 不代表文件系统稳定。大文件/全树复制受资源预算限制，超限明确阻断，不缩减快照后通过。成功绑定的是该封存内容（审计模式仍受第 2 节信任前提限制），不是对未来可变工作目录的永久保证；外部任意进程在提交后的修改不属于已验收结果。scope 门禁与 canonical Run 投影保持同一业务结果，不留下 Loop failed 但 canonical done 的冲突。

### 5.3 产物检查的覆盖

baseline 与每个阶段快照覆盖 worktree 所有条目：tracked、untracked、ignored、隐藏、删除、重命名两端、文件类型/权限、symlink/junction 元数据和二进制内容摘要。UI 的 bounded Git diff/status 仅用于展示，不能作为证明输入。严格模式的内容基线和清单必须由实际执行边界保证 Agent 不可写，而不只是放到另一个目录；审计模式在第 2 节的宿主控制面可信前提下封存并核对证据，完整性异常一律 unverifiable，不宣称抗同 UID 主动篡改。扫描不能跟随链接读出范围外内容。

普通测试输出、缓存和生成物仍是变更，必须在允许范围内；需要独立临时目录时，只分配明确、窄化、逐 run 回收的执行器资源，并纳入能力评估，不能把宿主根目录或任意 cache 当默认豁免。受保护资源的扫描权限错误、文件持续变化、预算溢出、取消或不支持的元数据均产生 `unverifiable`，不能跳过后通过。扫描任务异步、可取消、有限资源，但不能通过截断伪造完整性。

verification 证据复用键至少包括 command definition hash、输入/工作区指纹、scope digest、executor witness 和相关策略修订。仅命令 id 相同不够。复用也必须重新验证当前最终状态与完整范围证据。

## 6. ACP 读取与批准交付

所有 ACP 会话统一处理：Allow 在资源身份安全校验后读取；Deny 返回错误且不排批准；健康 Ask 在交互式会话创建持久批准并等待，批准前不打开文件读取内容；无可用人工通道的 Ask 直接拒绝。策略/存储异常不能作为可直接执行的 Ask；错误恢复后必须重新完成健康评估。

给现有 InteractionKind 增加 `FileRead`（路径、行号/limit、资源及所有权引用，不含文件内容），覆盖 selector、`decision_fits`、`cancelled_reply`、`interaction_display`、tool_call_id、commit/delivery/ack/timeout 等匹配分支。先校验读取参数边界，再排批准；不发明第二套 pending store 或自有 TTL。

批准绑定 stable principal、run（如有）/session/generation/epoch/role、scope digest、真实资源身份和策略修订。复用现有单赢家 resolution claim、提交后交付与 ack；一次批准不能复用到另一个目标、运行或 generation。交付前核对取消、过期、run ownership、scope、路径替换、当前 Deny 和评估健康状况；如果仍为健康 Ask，已提交且匹配的 Once 批准允许本次交付，不得陷入无限再次 Ask。资源在等待期间被替换时拒绝旧请求并要求重新发起。

Deny、取消、超时、故障、重启、重复/迟到 resolution 均不得返回文件内容；重复交付不得产生第二次 effect。沿用原有记忆 grant 存储语义，但任何 Session/Project/Global grant 都不能绕过运行范围。

## 7. 持久化、恢复与状态

使用仓库现有下一可用 migration id，不复写旧 migration。新增版本/绑定/证据引用按实际 repository 设计采用 additive migration；写入通过 `SqliteWriteTransaction::Immediate`、唯一约束与 CAS 等既有机制保持一致性，不用先查再写模拟排他。

旧定义保留读取/编辑能力，显示 `legacy-unverified`；只有显式编辑为合法 v1 范围并选择模式后才能启动，空 allowed 不隐式转换为整个项目。旧非终态 run 缺绑定时暂停并显示 `scope-binding-missing`，不得重新构造假想旧授权后继续。用户可以取消旧 run、从已确认定义另开新 run；保留历史终态原状态并注明无新范围证明。

| 发现 | 结果 | 恢复条件 |
|---|---|---|
| 已确认保护/越界变更 | Failed，`scope-violation` | 不能 accept/continue 为成功；保留 worktree/证据，修订配置后另开运行 |
| 扫描不完整或根身份不确定 | Paused，`scope-unverifiable` | 修复原因并在同一有效范围内重建证据 |
| 缺少历史绑定 | Paused，`scope-binding-missing` | 该旧 run 不续跑；取消并从确认后的定义新开 |
| provider/工具能力或权限变化 | Paused，`scope-capability-changed` | 重新证明仍满足冻结模式/上限；不能自动降低模式 |

扩展 domain 对 Paused/terminal reason 的合法组合，避免现有 invariant 将新增暂停原因拒绝。resume/continue/control 都经过同一检查，不得先清理由来不明的 blocker。重启消费现有 shared recovery projection，不重放可能已经生效的工具；恢复范围证据后才能安排下阶段。审计模式恢复须有新的人工确认且不扩大原范围。

## 8. UI、日志与兼容

表单明确“允许修改路径”“禁止修改路径”和字面路径示例，review 展示规范化范围、模式和限制；模式/范围字段在编辑、复制、enable/disable、Web mock roundtrip 保留。preflight 的 `passed/blocked` 保留作为能否启动，另有常显覆盖评估；即便 passed 也展示未覆盖通道，避免绿色 readiness 暗示隔离。

run header 展示持久化的本次模式、覆盖程度和证据状态；不能用当前 Agent 新能力重标历史运行。timeline/detail 展示具体原因与可操作的恢复步骤。产物审计确认只在模式实际选中时出现，确认绑定后端当前版本；Web 模拟明确标注，能演示拒绝/Ask/迁移/过期，不声称本地执行。

所有新增用户文案同时提供简体中文/英文，复用既有主题、布局和可访问性。日志继续走 unified operations：存储前脱敏，只保留有界资源引用、动作、原因和证据摘要，不记录 ACP 文件内容或原始敏感工具输入，不新增 feature-local log。

## 9. 实施取舍

- 选择真实资源边界而非字符串前缀：成本更高，但避免 symlink/rename/TOCTOU 绕过。
- 选择运行上限快照与动态策略复核：保证配置可审计，同时使撤权仍生效。
- 保留显式产物审计兼容模式：减少现有 CLI 立即不可用的影响，但绝不把它显示成执行前约束，也不降低 Verifier 只读标准。
- 选择完整快照而非 Git diff：覆盖 ignored/untracked/元数据，代价是扫描 IO；通过异步/预算/缓存优化，完整性不足仍阻断。
- 不扩展成通用沙箱项目：交付托管 Worker、内置 verification、只读 Verifier、最终 accept 的完整正路径，显式拒绝未知通道，平台能力随后独立增强。

## 10. 验证策略

详见 acceptance.md 的矩阵。真实临时 worktree 与安全 sentinel 验证文件是否被修改/返回，不只断言 evaluator 返回值。测试路径中不使用用户真实凭据或远程服务。区分 PASS、FAIL、BLOCKED、NOT RUN；未覆盖平台不得宣称完成其强制能力。实际实施结束执行 tasks.md 的仓库门禁；本次文档生成只验证 OpenSpec 与引用一致性。
