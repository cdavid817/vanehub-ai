# Tree-sitter 代码索引

工作区代码会被 Tree-sitter 解析成有边界的、带类型的 chunk 与 symbol。这是 `retrieval` bounded context 的本地那一半——它无需任何外部服务即可运行,并在 embedding 被确认之前就使 FTS 可用。共享的宿主级内存池是另一个独立的关注点,见 [检索与向量搜索](retrieval.md)。

## 准入代码与错误容忍

只有准入了的代码会被解析,每种语言使用各自选定的 Tree-sitter grammar。解析是容忍错误的:当某个文件包含语法错误时,系统只会索引从错误周围的有效具名子树派生出的、有边界的 chunk。错误子树不会被索引。

## 有边界的带类型 chunk

每个 chunk 持久化时附带:工作区 id、归一化的相对路径、语言、行范围、symbol 名称、symbol 种类、chunk key 与索引版本。symbol 定义元数据(例如函数或类定义的名称、种类和定义范围)在同一文件事务中持久化,因此某个 symbol 可连同其 chunk 一起被发现。

## Chunk 预算与拆分

单个大于所配置 chunk 预算的 symbol 会被拆分成多个 chunk。拆分后的每个 chunk 仍然能归属到其源 symbol 与文件范围。

## 持久化前的脱敏

统一的敏感信息策略会在任何 chunk 文本被持久化、embedding、记录日志、审计或从 `search_code` 返回之前,应用于已准入的代码。原始代码内容不会被复制到检索存储中。包含敏感值的 chunk 会带上一个脱敏标记,而不是带上该值本身。

## 索引版本与陈旧

一个代码索引版本涵盖 grammar 兼容性、Tree-sitter 查询、chunk 拆分与脱敏策略。版本不匹配会把受影响的工作区文件标记为陈旧,并以有边界的批次重建。native worker 执行元数据优先的核对,只读取或解析新增或变更的文件。

## 设计所在

本章用于为贡献者定向。权威的需求位于规范中。

- [openspec/specs/workspace-code-indexing](../../../../openspec/specs/workspace-code-indexing/spec.md)

拥有此部分的 `retrieval` bounded context 在 [Native bounded context](native-contexts.md) 中描述;共享内存池那一半在 [检索与向量搜索](retrieval.md) 中。
