# OpenSpec 工作流

新功能与架构变更在实现之前先从 OpenSpec proposal 起步。

```mermaid
flowchart TB
  EX["1 · 探索问题<br/>检视既有主规范"] --> CR["2 · 创建命名变更<br/>proposal · design · delta specs · tasks"]
  CR --> VS{"3 · openspec validate<br/>&lt;change&gt; --strict"}
  VS -->|"不通过"| CR
  VS -->|"通过"| AP["4 · 逐项应用任务"]
  AP --> FV{"聚焦验证通过?"}
  FV -->|"否"| AP
  FV -->|"是"| CK["勾选该任务的复选框"]
  CK --> MORE{"还有未完成任务?"}
  MORE -->|"有"| AP
  MORE -->|"没有"| FULL{"5 · 完整项目校验套件"}
  FULL -->|"失败"| AP
  FULL -->|"通过"| VER{"6 · 依据工件验证实现"}
  VER -->|"对不上"| AP
  VER -->|"一致"| ARC["7 · openspec archive<br/>重新生成归档索引"]
  ARC --> CM["主 specs + 归档目录 + 索引<br/>一并提交"]
```

**第 4 步那个回路是这套流程的重点**：复选框不是「我打算做」的清单，而是「这一项已实现且已通过聚焦验证」的记录——先勾再做，整条链的证据就失效了。

1. 探索问题并检视既有的主规范。
2. 创建一个命名变更,包含 proposal、design、delta specs 与 tasks。
3. 运行严格的变更校验(strict change validation)。
4. 应用各项任务,只有在某项任务实现完成并通过聚焦验证后,才勾选其对应的复选框。
5. 运行完整的项目校验套件。
6. 依据工件(artifacts)验证实现。
7. 归档变更,重新生成归档索引,并将主 specs、归档目录与索引一并提交。

`openspec/specs` 下的主规范是行为的唯一真源。已归档的 Markdown 工件仍以在线形式保留在 Git 中;压缩归档不能替代它。

在打开单个工件之前,先用 `openspec/changes/archive/archive-index.json` 定位历史变更。
