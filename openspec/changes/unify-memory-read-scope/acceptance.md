# 读取范围验收矩阵

全部运行用例当前 **NOT RUN**。后续用真实临时 v2 Markdown、SQLite 和固定 embedding/provider fixture 实现；每个场景用唯一 sentinel 检查 selector 输入、provider 请求、工具结果、Context Engine 投影和日志。不得只断言权限函数或全部空结果。

四入口简称：I=索引/摘要注入，B=选择正文，R=recall，C=Context Engine。对静态 corpus、相同 context，预算前 eligible IDs 应相同；各入口的最终排序和数量可不同。CLI 仅测其声明支持的 I，不能要求未实现的 CLI recall。

## 公共测试数据

Agent A、B；本地 workspace W1、W2；同仓库 worktree WT1/WT2；同路径但不同连接的远程 R1/R2。记忆至少包括：G-all、G-A、W1-all、W1-A、W1-B、W2-all、candidate、archived、deleted、malformed、quarantined，以及同名不同 ID/revision/hash 的两条记录。来源作者刻意与 audience 不同，防止把作者当授权。

| ID | 条件 | 必须结果 | 任务 |
|---|---|---|---|
| MR-01 | standard/A/W1，read+global 开启 | I/B/R/C 可选 G-all、G-A、W1-all、W1-A；其余无摘要/正文泄漏 | 2.1,5.1–5.5 |
| MR-02 | standard/A/W1，global 关闭 | W1 合法记录仍成功；global 经四入口均不可见 | 2.1,4.3 |
| MR-03 | project-only/A/W1 | W1 合法正文可用，global/其他workspace不可见 | 2.3,5.2 |
| MR-04 | temporary 或 read disabled，embedding已配置、Context Engine开启 | memory source/selector/search/query embedding/body load 计数为0，模型生成仍完成 | 2.4,4.5,5.5 |
| MR-05 | standard 明确无 workspace；另一次 workspace解析失败 | 前者按global策略；后者全部memory拒绝，不能混成None后允许global | 2.3 |
| MR-06 | selected audience、作者不同，id含大小写差异/%/_/引号边界 | 只精确匹配成员；不按作者/子串/LIKE返回；Rust/SQL/Web集合一致 | 2.1,2.2 |
| MR-07 | scope/status未知、非法JSON、workspace字段组合损坏 | 在metadata进入selector/索引前排除；安全诊断，不猜测eligible | 2.2,3.2 |
| MR-08 | 同一repo的不同worktree、相同远程路径不同连接 | 使用既有稳定身份隔离；不按显示路径或父repo合并 | 2.3,6.1 |
| MR-09 | 模型伪造agentId/workspaceKey/scope/owner/ID列表；直接跳过catalog调用 | 不能改变native主体/调用owner管理；无context拒绝 | 1.4,2.5,5.4 |
| MR-10 | 同一群聊并发A/B seat、子调用换scope、结束generation重用 | 独立native context，不借用其他seat或失效授权 | 2.5,5.3 |
| MR-11 | 201条以上合法记忆，最相关目标在注入refs页之外 | R/C可找到，I仍如实truncated；不能把200条当ACL | 4.2,7.3 |
| MR-12 | 全局top-K全被不可读记录占据，合法命中排名靠后 | 两路均先授权再top-K，合法匹配不被越权项挤掉 | 4.3 |
| MR-13 | vector成功、FTS失败；FTS成功、vector失败；两路都失败 | 单路仍同一范围；degraded准确；双失败unavailable且生成继续 | 4.5 |
| MR-14 | 两路正常无匹配；资格关系构建中断/预算耗尽 | 前者空成功；后者零hits且unavailable，不能从半张关系返回partial或全池fallback | 4.2,4.4 |
| MR-15 | 权威文件archive/delete/audience缩小/跨scope迁移后索引残留 | 后续新交付拒绝，不能用旧FTS正文/索引标题兜底 | 3.2,4.4 |
| MR-16 | record revision/hash变化，或外部编辑不升revision、路径替换 | 旧handle丢弃，metadata/body一致；不偷换新版或新目标 | 3.2 |
| MR-17 | 冻结policy后用户更改read/global策略 | 本generation保持原policy语义，后续generation按新策略；界面不声称已撤回旧内容 | 2.4,6.5 |
| MR-18 | 新合法record在本generation中获批或修改 | 后续新read batch可按同一冻结规则pin新版本；已有旧handle不可被替换 | 3.1,3.2 |
| MR-19 | 两条同名合法记忆，selector选其中ID或编造ID | 精确选择对应ID，编造ID丢弃；不得取同名第一条/owner补查 | 5.2 |
| MR-20 | A已surfaced，切B/换workspace；revision更新；缓存锁故障 | 先重新判资格，去重不互相抑制或扩权；故障最多重复合法内容 | 5.3 |
| MR-21 | Context Engine required/protected片段或旧tool-result跨主体重注入 | 来源可识别memory重验，不靠显式引用/旧缓存抬升权限 | 5.5,5.6 |
| MR-22 | 合法workspace/selected-agent记忆首次保存与重建 | 进入本地FTS并可recall及加载正文，save不等embedding | 3.3–3.6 |
| MR-23 | restricted正文进入claim/retry/rebuild/模型切换；或公共项排队后改restricted | 基线无授权则FTS-only，所有队列路径及embed前复核阻止正文外发；不借代码索引确认、不计vector完成、不无限失败重试 | 3.5,3.7 |
| MR-24 | 不同模型embedding、pending记录、重建中断 | 向量仅同模型；关键词独立；中断可重入且不误删另一来源/工作区 | 3.6,4.3 |
| MR-25 | owner设置页列所有scope/归档/candidate；Agent试图用同接口 | owner管理照常；Agent不能用None context进入管理权限 | 1.4,3.3 |
| MR-26 | transient policy读失败有精确LKG；无LKG/迁移不健康/未知schema | 前者按原规则并warning，后者无memory；不继承其他主体缓存 | 2.4,2.5 |
| MR-27 | 允许读取但空池、未配置embedding、CLI仅index能力 | 三者与read disabled区别；注入不依赖embedding，CLI不冒充recall | 5.4,6.2 |
| MR-28 | bound-session预览伪造mode/workspace；hypothetical比较 | 真实值native解析；假设结果不能铸造runtime context | 6.1 |
| MR-29 | Web/native多层policy precedence、mode hard restriction、index预算 | eligible IDs和可用性一致，Web标记simulated；empty≠disabled | 6.3 |
| MR-30 | 源身份/资格失败、hidden标题、正文sentinel、原始query | 日志/manifest无原始内容；模型无排除记录存在性信息；owner计数仍可见 | 6.6 |
| MR-31 | query-local关系构建取消，连接池复用，A/B并发，metadata持续变化 | 完整性不足拒绝，关系回收且不串主体，无跨网络长事务 | 4.2 |
| MR-32 | 201/1,000/10,000条池，限20结果 | 不全量读取正文；分页与候选/内存有界，记录query plan及P50/P95，不伪造固定性能承诺 | 7.3 |

## 最小真实集成验收

必须完成一个使用真实 v2 文件、SQLite/FTS、固定 embedding 与 provider fixture 的 OnePiece generation：先snapshot，再Context Engine，再I/B，再模型调用R，最后正常完成。分别跑 MR-01 正路径和 MR-04 关闭路径，并证明 MR-03 工作区正文可用和 MR-11 超200可召回。固定模型输出可以模拟，文件/资格查询/索引/bridge/交付不能全部mock。

涉及页面的测试覆盖简中/英文、两主题、窄屏、空池和预览。Windows/macOS/Linux 工作区和文件一致性分别使用实际执行结果 PASSED/FAILED/BLOCKED/NOT RUN；Web模拟不代表原生通过。
