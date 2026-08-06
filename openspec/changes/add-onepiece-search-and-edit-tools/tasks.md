> **Task 10 回填状态（务必先读）**：Task 1-9 的实现与自审均已完成并落盘；Task 10 执行全量校验时，`cargo test --manifest-path src-tauri/Cargo.toml`（不带 `--lib`，即包含 `tests/architecture.rs` 等集成测试可执行文件）在本变更范围内**首次**被完整跑通，发现一个真实的、非环境抖动的架构边界回归：`edit_tool.rs`（Task 5 引入的原子写入实现）触发了 `runtime_processes_and_append_logs_use_shared_adapters` 这条既有架构适配测试。详见 §9.1。**在这一项被修复并重新验证之前，本变更不满足「全部通过」的归档前提，不应执行 `openspec archive`。**

## 1. 共享依赖与受限遍历（walk.rs）

- [x] 1.1 在 `src-tauri/Cargo.toml` 新增 `regex`、`ignore`、`globset` 直接依赖 —— 三者均为直接依赖（而非仅由 `ignore` 间接引入 `globset`）：`glob_tool.rs`/`grep_tool.rs` 都直接 `use globset::GlobBuilder`，`grep_tool.rs` 直接 `use regex::RegexBuilder`。commit `b03d0c5`。

- [x] 1.2 新增 `walk.rs`：`BoundedFilesystem` 边界 + `.gitignore`/`.ignore` 过滤 + 跳过符号链接 + 取消信号 + 结果上限的共享遍历实现 —— 实现 `visit_workspace_files(workspace_folder, relative_root, visitor, cancelled)`，基于 `ignore::WalkBuilder`（`git_ignore`/`git_global`/`git_exclude`/`parents` 全部 `true`，隐藏文件跳过），每个条目先过 `BoundedFilesystem` 边界校验再回调 `visitor: &mut dyn FnMut(&WalkedFile<'_>) -> Visit`。`walk.rs` 只对外暴露 `is_binary`（NUL 字节探测）与 `exceeds_size_limit`（`MAX_FILE_BYTES = 10MB`）两个原语供 Task 3-6 复用，行数/字节上限不在 `walk.rs` 里做——那是各工具自己的职责。`MAX_SEARCH_RESULTS`（200）与 `MAX_TOOL_OUTPUT_BYTES`（64KB）在 Task 2 自身审查中从 `walk.rs` 挪到了 `tools/mod.rs` 作为共享 `pub(crate) const`，同时删除 `shell_tool.rs` 里重复定义的 `SHELL_OUTPUT_LIMIT`，改指向同一常量。

  **回调 payload 的后续修正（Task 3 审查发现，修复提交在 walk.rs 而非 glob_tool.rs，详见 §2）**：初版回调签名是 `&str`（workspace-relative 的 `display`）+ `&Path`（`absolute`）。Task 3 审查时发现，一旦 `path` 参数收窄了搜索范围，未锚定的 glob pattern（如 `"*.md"`）永远匹配不到嵌套文件——因为 `globset::GlobBuilder::literal_separator(true)` 不允许 `*` 跨目录分隔符，而匹配对象仍是相对 workspace 根、而非相对收窄后子目录的 `display`。修复后回调改传 `WalkedFile<'a> { absolute, display, scoped }` 结构体（刻意用具名字段而非三个同类型的 `&str`/`&Path` 位置参数，避免两个字符串字段被误换后编译器却无法发现）：`scoped` 相对"搜索范围根"计算（`relative_root` 收窄时为该子目录，否则等于 workspace 根，二者用同一段代码算出，无需分支），供 pattern 匹配使用；`display` 保持 workspace 相对，是唯一允许出现在工具输出、返回给模型的路径形式。

- [x] 1.3 单元测试覆盖遍历的忽略规则、符号链接跳过、取消、上限行为 —— 最终 `walk::` 15 个测试（Windows 上可编译的部分）：隐藏条目跳过、`.gitignore` 跳过（含无 git 仓库场景）、`relative_root` 收窄与越界拒绝、取消后中止、文件/目录符号链接指向 workspace 外时不访问（`_when_supported` 命名，Unix/Windows 各一对）、大小限制（含恰好等于上限不拒绝的边界用例，`File::set_len` 造稀疏文件而非真实写 10MB+1）、`scoped`/`display` 在有无 `relative_root` 两种场景下的等价/差异断言。Task 2 自身审查（9 项发现，全部修复，未削弱任何边界逻辑）额外做了：取消标志参数类型从 `&Arc<AtomicBool>` 改为 `&AtomicBool`（与 `api_process_adapter.rs` 既有签名一致）；纠正一处关于 `resolve_existing(".")` 的错误注释；为 Windows 扩展长路径（`\\?\` 前缀，`canonicalize()` 返回该形式，绝不能流入工具输出）与 `parents(true)`+`git_global(true)` 可能导致"静默返回空结果而非报错"两点补充文档；取消检查从 `Ordering::Relaxed` 改为 `SeqCst`（与其余取消检查点一致）；全部中文注释译为英文。

  **未能完全验证的一点（如实记录）**：新增的 Windows 目录/文件符号链接跳过测试在本机编译并通过，但只走到了"无操作（no-op）"分支，不是真正的断言分支——本机开发者模式未开启、shell 未提权，`mklink` 本身会因权限不足失败，符号链接从未在本机被真正创建过。这与仓库里 `platform/filesystem/mod.rs` 同类先例测试的既有限制一致，不是本次改动引入的新缺口，但也不是本次改动真正验证过的路径；依赖 CI 里权限更高的 runner 或 macOS/Linux lane 才能命中断言分支。

## 2. `glob` 工具

- [x] 2.1 新增 `glob_tool.rs`：按文件名模式匹配，复用 `walk.rs` —— `execute_glob(pattern, path, workspace_folder, cancelled)`：空/全空白 pattern 在构造 matcher 前即拒绝；非法 glob 语法报错并点名具体 pattern；匹配结果按 `MAX_SEARCH_RESULTS`（200）截断、排序后换行拼接，命中上限时显式追加截断提示；空匹配集返回非错误的 "No files matched" 提示而非报错。只使用 `display` 路径（workspace 相对）作为输出，从不使用 `absolute`。commit `e67df43`。

  **审查发现并修复（commit `07df921`，2 项 Important，均在 walk.rs / glob_tool.rs，未触碰边界本身的包含/符号链接/`.gitignore`/取消/大小限制逻辑）**：
  1. **`path` 收窄搜索时的静默假阴性**（根因与修复见 §1.2 的 `WalkedFile` 记录）——复现用例 `execute_glob("*.md", Some("docs"), <workspace 含 docs/guide.md>, ...)` 修复前返回 `is_error=false, output="No files matched \"*.md\"."`，而该文件确实存在。模型读到"未匹配"会误判文件不存在从而放弃搜索，是搜索类工具能产生的最坏一类错误。修复后改为匹配 `file.scoped`、输出 `file.display`。
  2. **截断测试未验证真实阈值**——`exceeding_the_result_limit_reports_truncation` 原先只断言 `output.contains("truncated")`，无论实际阈值有 off-by-one 还是翻倍错误，210 个候选文件下都会命中该字符串。改为拆分匹配区与提示区，对匹配区做 `assert_eq!(files.lines().count(), MAX_SEARCH_RESULTS)`，才是真正钉住上限的断言。

- [x] 2.2 单元测试覆盖匹配与忽略规则行为 —— 最终 9 个 `glob_tool::` 测试：brief 给定的 7 个（缺失 workspace 目录、非法/空 pattern、无匹配非错误、按扩展名匹配、`path` 收窄、截断提示）+ 上述审查新增的 `a_path_scope_matches_unanchored_patterns_against_the_narrowed_root` + Task 4 审查为对称性一并补上的 `a_result_count_exactly_at_the_cap_is_not_reported_as_truncated`（见 §3）。`.gitignore` 跳过行为复用 `walk.rs` 既有覆盖，未在 `glob_tool.rs` 里重复。

## 3. `grep` 工具

- [x] 3.1 新增 `grep_tool.rs`：`pattern`/`glob`/`path`/`output_mode`/`context`/`case_insensitive`/`head_limit` 参数与三种 `output_mode` —— `execute_grep(GrepRequest<'_>, workspace_folder, cancelled)`，`output_mode` 为 `files_with_matches`/`content`/`count` 三选一，先校验 `pattern` 非空与 `output_mode` 合法值再编译任何东西；过滤匹配 `file.scoped`（而非 `file.display`，吸取 §2 的教训）；`head_limit` 只能把 `MAX_SEARCH_RESULTS`（200）调低（`.unwrap_or(MAX_SEARCH_RESULTS).min(MAX_SEARCH_RESULTS)`），不能调高；`files_with_matches` 模式通过 `Iterator::any` 短路，命中即停，不扫描整个文件。commit（初版）`40fa226`。

  **性能缺陷与修复（commit `62ca083`，审查发现的 Critical #1，在本机真实基准测量，不是估算）**：`render_file` 的上下文行（`context`）展开原实现是 `wanted.contains(&index)` 在循环里逐次线性扫描一个不断增长的 `Vec`，是 O(n²)；`execute_grep` 的结果上限（`MAX_SEARCH_RESULTS`/`MAX_TOOL_OUTPUT_BYTES`）当时是在 `render_file` **整个物化返回**之后才检查，完全无法提前掐断这个开销。用同一份 10 万行全命中的数据集、同一台机器、同一次 `cargo test` 调用里测得：修复前 `context=0` 耗时 **76.1185898s**、`context=1` 耗时 **217.1074082s**；改为线性 merge-cursor（游标随已升序排列的命中位置单调推进，`wanted.extend` 只追加严格新增、已升序的下标，`sort_unstable()` 因此变为死代码并删除）后，`context=0` 耗时 **226.3215ms**、`context=1` 耗时 **217.9831ms**——约 336x / 996x。新增一条永久保留、默认 `#[ignore]`（不拖慢日常 `cargo test`）的回归基准 `context_expansion_scales_linearly_not_quadratically`，对 10 万行数据在 `context=1` 下设 1 秒硬性上限（实测约 220ms，约 5 倍余量），未来若性能回归会直接失败而不是静默变慢。

  同一轮审查另修复 8 项（含 5 项 Important）：字节上限原先是"先 push 再检查"，一行 3MB 的压缩代码可以冲破 64KB 预算，改为按剩余预算逐行截断（`truncate_line`，按 UTF-8 字符边界切，避免截断落在多字节字符中间）；`scoped` 与 `display` 此前没有任何测试能区分二者（现存用例全部 `path: None` 时两者恒等），补了一条 `path: Some("docs")` + 未锚定 glob 过滤器的用例同时钉住两个方向；`head_limit` 能否真正调高上限此前无测试；未校验的 `context` 直接喂入 `hit + context` 的无溢出保护加法，补 `MAX_CONTEXT_LINES = 20` 硬夹紧；`pattern.trim()` 会静默改写正则本身（`", "` 被裁成 `","`），改为只在"是否提供了内容"的判断上 `trim()`，正则编译用原始未裁剪值；截断提示在"结果数恰好等于上限、其实什么都没截断"时也会误报（"狼来了"），改为把上限检查挪到 push 之前，只有真正观察到下一个候选时才判定截断（grep 与 glob 都改）。

  **后续 mutation-testing 复查又发现并修复 2 项（同一提交序列内，见 §9 的"跨任务复盘"）**：`an_absurdly_large_context_does_not_panic_and_is_clamped` 的夹具把命中放在第 0 行，导致 `0 + usize::MAX` 无论有没有 `MAX_CONTEXT_LINES` 夹紧都不会溢出——即该测试在夹紧逻辑被删除后仍然全绿，未真正钉住它所声称覆盖的护栏；把夹具改到命中不在首行后，删除夹紧逻辑即可复现 `attempt to add with overflow` panic，确认测试真正生效。`head_limit: Some(0)` 原先会命中"No matches"提示，与真正无匹配无法区分，模型可能误判 pattern 在仓库里根本不存在；改为显式拒绝 `head_limit: 0`（与本文件里空 pattern/非法正则/非法 glob/未知 `output_mode` 的既有"拒绝而非静默兜底"惯例一致）。

- [x] 3.2 接入 10MB 输入上限（静默跳过超限文件）与二进制内容跳过 —— `fs::read` 前先用 `exceeds_size_limit(file.absolute)` 判定，超限/二进制/非 UTF-8 的文件静默跳过（不中断整个搜索），与 `edit`/`file` 读取"报错而非跳过"的语义刻意不同——搜索工具面对的是整个仓库，调用方没有点名某个具体文件。

- [x] 3.3 单元测试覆盖三种 `output_mode`、`.gitignore` 跳过、超限文件跳过 —— `grep_tool::` 测试数随三轮修复持续增长：brief 给定 14 个 → 审查修复（commit `62ca083`）新增 13 个（`tools::` 汇总从 48 增至 61 通过 + 1 个新增的永久 `#[ignore]` 基准，共 62 collected）→ mutation-testing 复查（commit `6b70b20`）再新增 1 个（`a_head_limit_of_zero_is_rejected_rather_than_reported_as_no_matches`，`tools::` 汇总变为 62 通过 + 1 ignored，63 collected）。三种 `output_mode`、`.gitignore` 跳过（复用 `walk.rs` 覆盖）、二进制/超限文件静默跳过均有专门用例；另有多字节字符（CJK，`truncate_line` 按字符边界切）截断用例，是 Task 6 审查里为对称性一并加到本文件的。

## 4. `edit` 工具

- [x] 4.1 新增 `edit_tool.rs`：`old_string`/`new_string`/`replace_all` 唯一匹配语义 —— `execute_edit(path, old_string, new_string, replace_all, workspace_folder)`：空 `old_string` 与 `old_string == new_string` 提前拒绝；`text.matches(old_string).count()` 为 0 报错"未找到"，大于 1 且 `!replace_all` 报错并给出**实际出现次数**（从不静默改第一处）；`replace_all` 用 `text.replace`，否则 `text.replacen(..., 1)`。commit（初版）`3384c1b`。

  **写入是原子的（临时文件 + `fs::rename`），这是相对 `fs::write` 的一次刻意的、已记录在案的语义变化**：`write_atomically` 先写到同目录的隐藏兄弟临时文件（`.{文件名}.edit-tmp-{pid}-{序号}`，文件名截到前 60 个字符以免加上后缀超出 NTFS 255 字符限制），尽力拷贝原文件权限，再 `fs::rename` 覆盖目标——Windows 上是 `MoveFileExW`（`MOVEFILE_REPLACE_EXISTING`）、POSIX 上是 `rename(2)`，两者都是原子操作；任何一步失败都清理临时文件，目标文件在失败路径上保持原样不变。**`rename` 覆盖已存在文件会断开硬链接**——若目标文件在别处有硬链接双胞胎，那个双胞胎会保留旧内容而不是跟着一起变。这被视为更好的默认行为（避免一次编辑意外改写一个硬链接式 `node_modules` 存储库里的全局共享文件），但明确是相对 `fs::write` 原地覆盖的语义改变，已写入 `write_atomically` 的文档注释，不是未预期的副作用。

  **Windows 锁定文件的两种系统错误码，覆盖程度不同（如实记录）**：本机真实复现"另一进程持有目标文件"这一条件（`OpenOptions::share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)`，刻意不给 `FILE_SHARE_DELETE`）时，测得的是 `PermissionDenied`（**os error 5**，`Failed to write "code.rs": 拒绝访问。`）。`ERROR_SHARING_VIOLATION`（**os error 32**）这条分支——`std` 不会把它映射到任何具体 `ErrorKind`，需要靠 `raw_os_error() == Some(32)` 才能识别——**只有一条用合成的 `io::Error::from_raw_os_error(32)` 构造的单元测试覆盖**（`a_sharing_violation_write_failure_reports_the_same_stable_message`），本次会话中从未在真实文件系统上复现过 os error 32 本身；两者都会走向同一条"文件可能被占用，未修改"的稳定英文提示。`#[cfg(unix)]` 门控的两个测试（`an_edit_preserves_the_original_file_permissions`、`the_temp_file_is_created_at_a_private_mode_not_widened_after_the_fact`）在本机（Windows）从未被编译执行过，依赖 `.github/workflows/ci.yml` 的 `macos-latest` lane 才能跑到。

- [x] 4.2 接入 10MB 输入上限（报错而非静默跳过）—— 与 `grep`/`glob` 的静默跳过刻意不同：调用方点名了这一个具体文件，静默跳过会让调用方误以为编辑发生了。额外补了"编辑结果投影大小"上限（`projected_replacement_len`，用 `saturating_add`/`saturating_mul`/`saturating_sub` 防止对抗性 `new_string` 整数环绕后看起来"没超限"），在真正调用 `replace`/`replacen` 之前算好投影字节数并拒绝——原实现是无条件分配 `text.len() + occurrences × (new_len − old_len)` 的字符串，一次 `String` 分配失败会让整个 Tauri 进程 abort，不只是这一次工具调用失败。

- [x] 4.3 单元测试覆盖 0/1/多匹配三分支与超限文件报错 —— brief 给定 9 个 → 实现方自行 mutation-testing（逐一临时禁用每个护栏、重跑测试、确认真的会失败）又补 3 个、加强 2 个 → 三轮外部审查（commits `df56447`、`b31d722`）分别新增 3、5 个（在本机 Windows 上可见的数量；另有 2 个 `#[cfg(unix)]` 用例未在本机编译），最终 `edit_tool::` 测试数在本机可见 24 个（`tools::` 汇总从 63 增至 82 通过 + 1 ignored 不变）。

  **实现方自查阶段就发现 brief 给定的 9 个测试里有 2 个是"伪阳性"**（护栏被临时禁用后，测试仍然全绿，因为同一夹具上**另一个**护栏顶替着报了错，掩盖了被测护栏本身的缺失；详见 §9 的"跨任务复盘"）：`an_empty_old_string_is_rejected`——空 pattern 在 `"内容\n".matches("")` 下会数出 9 次匹配，落进"匹配次数 >1"分支，`is_error` 同样为 `true`，只断言 `is_error` 分辨不出"因为空被拒"还是"因为匹配 9 次被拒"；`a_file_over_the_edit_size_limit_is_rejected_without_writing`（brief 之外补的一个真实存在的护栏）——用 `File::set_len` 造的稀疏文件读回全是零字节，零字节恰好是 `is_binary` 判定的 NUL 标记，大小护栏被删掉后 `is_binary` 会顶替着报错，`is_error` 同样看不出区别。两处都改为断言具体错误文案。

  **第二轮外部审查又发现"匹配次数"本身完全未被钉住**：`outcome.output.contains('3')`/`.contains('2')` 在把 `{occurrences}` 改写成 `occurrences + 10` 后依然通过（"13 次"里含 '3'，"12 次"里含 '2'）——而这个次数是这个工具的核心契约（防止"以为只改了一处，实际改了很多处"）。改为断言完整短语 `"matches 3 times"` / `"Replaced 2 occurrence"`。同一轮还发现路径越界测试用的 `../outside.rs` 根本不存在，测试在 `canonicalize` 阶段就已经失败，从未真正走到越界检查那一步——把越界检查整个删掉，这条测试依然会因为"文件不存在"而报错，`is_error` 看不出区别；改为让 `../outside.rs` 真实存在（workspace 的同级兄弟文件），越界检查被删除后确实还会报错，但报的是不同的 `OutsideRoot` 信息而非越界专属信息，从而让测试真正对特定护栏敏感。

## 5. `file` read 边界

- [x] 5.1 `file_tool.rs` 的 read 操作增加 `offset`/`limit` 分页与行号前缀 —— `execute_file` 签名增加 `offset: Option<usize>`、`limit: Option<usize>` 两个参数（4 参 → 6 参）；输出前缀 `"{1-indexed 行号}\t{内容}"`。commit（初版）`26e8441`。

  **计划外但为使 crate 能编译而必须的修复**：`api_process_adapter.rs` 的 `FILE_TOOL_NAME` 分发分支是 `execute_file` 在 `file_tool.rs` 之外**唯一**的生产调用点，计划里没有把它列进 Task 6 的文件清单，签名一变整个 crate 编译失败。当时先用 `execute_file(operation, path, content, None, None, folder)` 把编译修好（`offset`/`limit` 的真正 JSON 解析与 schema 声明留给 Task 7/8），同时修了该文件里两条硬编码旧行为（`assert_eq!(outcome.output, "hello")`）的既有测试，改为 `"1\thello"`。这是本变更过程中确认的第一处计划缺陷：**计划把这个调用点的更新放在了 Task 7，但它是 Task 6 signature 变更后立即断编译的阻塞项**，不改不行。

- [x] 5.2 接入行数/单行字符/总字节三档硬上限（`limit` 只能调低不能调高）与 10MB 前置大小检查（报错）—— `MAX_READ_LINES = 2000`、`MAX_READ_LINE_CHARS = 2000`、共享的 `MAX_TOOL_OUTPUT_BYTES = 64KB`。字节上限的检查顺序**刻意偏离了 brief 的字面代码**：brief 是"先 push 再检查"（与 grep 当时被发现的问题同一形状），实现时改用 grep 修复后的"push 前检查 + 单条目按剩余预算截断"（`truncate_entry`，与 `truncate_line` 同构）。用临时改回 brief 字面写法的方式做了实证：不做单条目截断时，新增的 `the_output_byte_budget_is_enforced_before_pushing_and_stays_bounded` 测试会以超出预算 663 字节的方式失败，其余 20 条测试仍然全绿——证明只有这一条测试真正钉住了这处修复。

- [x] 5.3 二进制内容拒绝（NUL 字节判定，返回明确原因而非 UTF-8 解码错误）—— 复用 `walk::is_binary`，与 `edit_tool` 同样的"报错而不是静默跳过"语义（调用方点名了具体文件）。

- [x] 5.4 单元测试覆盖分页、行号、三档上限、二进制拒绝、超限文件报错 —— brief 给定 6 个新测试 + 6 个既有测试更新调用签名 → 又补 9 个（覆盖 brief 6 个测试完全没碰过的 `exceeds_size_limit`/UTF-8 护栏、`>=` 与 `>` 的精确边界、空文件、`limit` 超过硬上限不能放大等），初版 `file_tool.rs` 从 6 个测试增至 21 个（`tools::` 汇总 82 → 97 通过 + 1 ignored）。审查（commit `a3cec8a`）又发现并修复 6 项（1 Important + 5 Minor），新增 6 个 `file_tool` 测试 + 1 个 `grep_tool` 多字节测试（`tools::` 汇总 97 → 104 通过 + 1 ignored，此后保持到 Task 10 结束未再变化）：单行字符上限的 `>` vs `>=` 边界此前无测试能区分（用 mutation 验证：改成 `>=` 后 26/27 条仍然全绿，只有新增的边界测试会失败）；`limit: Some(0)` 此前静默返回空输出且提示引用不存在的"第 0 行"，改为与 grep 的 `head_limit: 0` 同款显式拒绝；截断提示没说清"从哪个 offset 续读"，且把 1-indexed 的"最后一行行号"和 0-indexed 的"续读 offset"两种进制混在一起，模型只能猜；单条目截断恰好发生在文件真正最后一行时，旧提示仍然建议一个不存在下一页的 offset，改为按 `more_remains` 分支给出不同措辞；两个文件的截断路径此前都没有多字节字符夹具；空文件 + 非零 offset 会被 `&&` 短路成"看起来合法"，改为显式两分支判断。

## 6. 风险分级、信任白名单与工具路由

- [x] 6.1 `risk_tier_for()`：`grep`/`glob` 归为 `AutoApprove`，`edit` 归为 `RequiresApproval` —— `edit` 显式列了一个独立分支（而不是落到默认的 `_ => RequiresApproval`），带注释说明这处"冗余"是刻意的；`cargo clippy --all-targets` 未把它当成 `match_same_arms` 报警。commit（初版）`5a7c1ab`。

- [x] 6.2 `requires_approval()` 信任白名单加入 `edit` —— 白名单条件从 `shell || file` 扩为 `shell || file || edit`，同步更新了函数文档注释（原先说"只能免批 shell 和 file"，不再是事实）。

- [x] 6.3 `execute_tool_call()` 路由新增 `grep`/`glob`/`edit` 三个分支 —— 同时顺手把 Task 6 遗留的 `execute_file(..., None, None, ...)` 占位改成真正从 JSON `input` 解析 `offset`/`limit`。发现一个不在 brief 文件清单里但结构性必需的修复：`tool_catalog.rs` 里新增的常量是 `pub(crate)`，但 `api_process_adapter.rs` 实际是通过 `application/mod.rs` 的聚合 `pub(crate) use tool_catalog::{...}` 重导出来访问它们的，不是直接从 `tool_catalog` 导入——`EDIT_TOOL_NAME`/`GREP_TOOL_NAME`/`GLOB_TOOL_NAME` 需要一并加进那个重导出列表，否则报"存在但不可访问"而非"不存在"。

- [x] 6.4 `execute_tool_call()` 的 plan mode 硬拒逻辑新增 `edit` —— 拒绝检查加在 `EDIT_TOOL_NAME` 分支真正触碰文件系统之前、也在 workspace-folder 校验之前，是唯一的生产分发入口。测试不仅断言 `is_error`，还额外读回夹具文件内容，逐字节确认"被拒绝的调用确实什么都没改"。

- [x] 6.5 单元测试覆盖风险分级、信任白名单、plan mode 硬拒（含模型主动请求 `edit` 的场景）—— 初版 `tool_catalog` 相关测试 21→25（+4：新工具风险分级、`edit` 恒需批准、受信任 agent 免批 `edit`、信任不外溢到 MCP 工具），`api_process_adapter` 相关测试 71→74（+3：三个新分支的路由、`edit` 在 plan mode 被拒、search 工具在 plan mode 仍可用）。

  **审查（4 项发现，均未削弱任何护栏）**：Finding 1（Important）——`search_tools_do_not_require_approval` 测的是 `risk_tier_for`，不是生产真正调用的 `requires_approval`（后者还叠加了per-agent 信任标志）。用 mutation 证实：把 `requires_approval` 改到对 grep/glob 总是要求批准，`risk_tier_for`-only 的测试仍然全绿，只有扩展后同时覆盖两个工具、经过 `requires_approval` 的既有 `trust_flag_never_affects_already_auto_approved_tools` 会失败——确认了扩展是有效的，不是摆设。Finding 2（Important）——`offset`/`limit`/`context`/`head_limit` 的 JSON 解析原先用 `Value::as_u64`，浮点数（如 `100.0`）、负数、字符串全部被静默解释成"参数缺失"而不是报错；新增 `non_negative_integer`/`parse_optional_non_negative_integer_arg` 两个辅助函数（先试 `as_u64`，失败再试 `as_f64` 并要求 `is_finite() && >= 0.0 && fract() == 0.0`），新增 12 个测试，其中一个直接复现了原始 bug：`limit: 3.0` 在修复前会被当成"未提供"从而读到 2000 行的默认值，而不是真正把读取限制在 3 行。Finding 3——删掉一条与既有测试完全重复的测试。Finding 4——`edit` 分支的路由测试原先只断言 `!is_error`，未验证编辑真的落盘，补了读回校验。

## 7. 工具目录开关

- [x] 7.1 `tool_catalog()` 从 3 个工具扩展到 6 个（`shell`/`file`/`remember`/`grep`/`glob`/`edit`）—— 新增 `grep_tool_definition()`/`glob_tool_definition()`/`edit_tool_definition()` 三个私有构造函数（`grep`/`glob` 的 schema 在完整目录与 plan-mode 目录间共享同一个构造函数，避免两份定义漂移）。同时把此前 Task 7 已经解析、但两个 schema 里都从未声明过的 `file` 工具 `offset`/`limit` 属性补进两份 `input_schema.properties`——此前是"解析器认，模型摸不到"的状态。commit（初版）`f0a7d1da`。

- [x] 7.2 `plan_mode_tool_catalog()` 从 2 个扩展到 4 个（+`grep`/`glob`）—— `shell`/`edit` 在这份目录里天然不存在（从未被加进对应 `vec![]`），不需要额外的排除逻辑。

- [x] 7.3 更新契约测试 `catalog_declares_exactly_shell_file_and_remember_tools`（`catalog.len()` 断言随之变化）—— 按 brief 替换为三条组合/顺序测试（`catalog_declares_the_six_native_tools_in_a_stable_order`、`plan_mode_catalog_offers_only_read_only_tools`、`plan_mode_catalog_never_offers_shell_or_edit`），另加 4 条本任务自行补的"完整参数面"测试（`grep`/`glob`/`edit`/`file` 各一条，逐一核对 `required` 列表与全部 `properties` 键名，不只是核对存不存在）。

  **计划缺陷 #2（本变更过程中确认的第二处）**：目录从 3 个扩到 6 个，破坏了 `api_process_adapter.rs`（3 条 `resolve_tool_catalog_*` 测试，硬编码的 `tools.len()` 与逐下标工具名断言）、**`anthropic_provider.rs`** 与 **`openai_compatible_provider.rs`**（各一条 `request_body_declares_tools_when_provided`，同样硬编码长度与下标）三个文件里一共 5 处硬编码测试。**计划把这次任务的文件清单只列了 `tool_catalog.rs`**，但这三个文件的测试都直接调用 `tool_catalog()`，长度一变就立刻编译期之外的运行期断言失败。修复只做了机械的数字/下标扩展（如 `[0..2]` 扩到 `[0..5]`），审查确认全部是**加强**（下标断言变得更细）而非放宽。tool_catalog 相关测试最终 29 个全绿（`tool_catalog.rs` 本身的 22 个 + `api_process_adapter.rs` 的 4 个 `resolve_tool_catalog_*` + MCP service 的 3 个 `visible_tool_catalog_*`，后两组是子串匹配带出来的、本任务未改动其逻辑）；`api_process_adapter` 相关测试维持 86 个不变（这次只改断言数字，没加测试）。

  **审查（5 项发现，1 项 Important）**：`file` 工具此前没有任何"完整参数面"测试——审查方实测把 `content` 属性删掉、`required` 从 `["operation","path"]` 弱化成 `["operation"]`，29 条测试全部还是绿的。改写 `file_tool_schema_declares_offset_and_limit_in_both_catalogs` 为 `file_tool_schema_declares_its_full_argument_surface_in_both_catalogs`，同时钉住两份目录的 `required`、完整属性集、以及 plan-mode 版本明确没有 `content` 属性；用同样的 mutation 复现确认新测试会失败。另外 4 项 Minor：plan-mode 的 `file` schema 里 `offset`/`limit` 描述文案抄了"Ignored when writing"，而 plan-mode 目录的 `operation` 枚举本来就只有 `read`——已删除该文案；`offset` 的描述原文是"Line number to start reading from (0-based)"，用"行号"这个 1-based 名词又配 0-based 括注，容易让模型把 grep 报出的 `file.rs:412:` 错当 `offset: 412`（实际会落到 413 行）——改成"0-based index of the first line to return"；`context`/`head_limit`/`limit` 的隐式夹紧此前完全没写进 schema 描述，现已补上具体数字（20/200/2000）。

## 8. Web mock grep 示例

- [x] 8.1 `web-agent-client.ts` 的模拟工具调用序列中补一个 `grep` 调用示例，与桌面能力保持演示保真度 —— 在既有 `shell` 审批弹窗（230ms）与 `remember`（235ms）之间插入一个 233ms 的 `grep` 模拟：直接以 `status: "completed"` 的单个 `tool_use` 事件发布（走 `remember` 的免批路径，而非 `shell` 的 `pendingMockToolApprovals`/`resolveToolApproval` 审批路径——这是本任务里唯一需要验证正确的属性，因为 `grep` 被分类为 `AutoApprove`）。固定假输出（`src/App.tsx`/`src/main.tsx`），Web 运行时本来就不触碰真实文件系统。定时器已注册进 `timeoutIds`，不会泄漏。未新增 `glob` 模拟（brief 未要求）。commit `3064f8c`。frontend 528/528 测试全绿，本次改动没有新增/修改任何测试用例。

## 9. Verification

- [ ] 9.1 `cargo test --manifest-path src-tauri/Cargo.toml` —— **未能全绿，发现一处真实的、非环境抖动的回归。** 这是本变更全程第一次运行不带 `--lib` 限定的 `cargo test`：Task 1-9 的每一次校验都只跑过 `--lib`（或再加 `tools::`/`tool_catalog`/`api_process_adapter` 模块过滤）范围，从未编译执行过 `tests/architecture.rs` 这个独立集成测试可执行文件。`--lib` 部分本身干净：**1358 passed; 0 failed; 10 ignored**（含已知偶发的 `contexts::tooling::mcp::infrastructure::relay::tests` 一组，本次运行未触发该抖动）。`src/main.rs` 0 tests。`tests/architecture.rs` 12 个测试里 1 个失败：

    ```
    thread 'runtime_processes_and_append_logs_use_shared_adapters' panicked at tests\architecture.rs:800:5:
    runtime I/O bypasses shared platform/operations adapters:
    contexts/agent_runtime/infrastructure/tools/edit_tool.rs:222: feature-local append-file construction
    test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
    ```

    根因：`edit_tool.rs` 的 `#[cfg(unix)] fn create_temp_file`（Task 5 第三轮审查为修复"临时文件短暂可被其他用户读取"而新增，commit `b31d722`）里用 `std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600)` 直接创建私有临时文件。`tests/architecture.rs` 的 `runtime_processes_and_append_logs_use_shared_adapters` 用 `syn::parse_file` 静态扫描全部源码（不会求值 `#[cfg]`，所以即便这段代码只在 Unix 上编译，扫描仍然会看到它），凡是调用路径以 `["new", "OpenOptions"]` 结尾、且所在文件不是 `platform/logging.rs` 或 `platform/private_relay_fs.rs` 的，一律判定为"feature-local append-file construction"违规——不检查是否真的调用了 `.append(true)`，规则本身比名字暗示的更宽。`platform/private_relay_fs.rs` 里已经有一个几乎同构的私有 `open_private_file()` 辅助函数（`OpenOptions::new().write(true).create_new(true).mode(0o600)`），`edit_tool.rs` 这次相当于重新发明了一遍而不是复用它，正是这条架构护栏想防止的那类重复。

    这是本次全量验证中确认的**第三处计划/实现缺陷**：Task 5 第三轮审查在加固原子写入时引入了这处违规，因为 Task 1-9 全程没有一次运行覆盖 `tests/architecture.rs` 的完整 `cargo test`，所以三轮审查都没有捕捉到。已复核确认可稳定复现（非计时/socket 类问题，是纯静态 AST 扫描，重跑两次结果完全一致），不属于 brief 预警的 `relay_tests`/`relay_streamable_http*` 已知抖动模式（本次运行这组测试反而全部通过），也不属于前端 vitest worker 池耗尽的已知模式。**按照本任务的执行要求，未尝试修复 `edit_tool.rs` 或放宽/修改 `tests/architecture.rs`——需要一个专门的后续任务，在 `platform/` 下暴露一个共享的"以私有权限创建新文件"原语（很可能是把 `private_relay_fs.rs` 的 `open_private_file` 提升为 `pub(crate)` 并复用，或在 `platform/logging.rs` 补一个等价项），让 `edit_tool.rs` 改为调用它，而不是自建 `OpenOptions`。**

- [x] 9.2 `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` —— 干净，0 警告，退出码 0。另外额外用 CI 实际执行的 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`（`.github/workflows/ci.yml:218`）复核，同样 0 警告——说明 §9.1 的失败是这条静态架构护栏特有的问题，不是 clippy 能捕捉的一类问题（`OpenOptions::new()` 本身不违反任何 clippy lint）。

- [x] 9.3 `npm run test` —— **528/528 测试通过，130/130 文件**，本次运行干净、一次到位（此前 Task 9 报告过的 vitest worker 池耗尽是同一会话里背靠背重跑多次触发的环境抖动，Task 10 只跑了一次，未复现）。

- [x] 9.4 `npm run build` —— 干净：`tsc && vite build && node scripts/check-frontend-chunks.mjs` 全部通过，"Verified 16 lazy frontend chunks; main static closure 105.3 KiB gzip."（chunk 体积预算检查脚本要求的输出）。

- [x] 9.5 `openspec validate add-onepiece-search-and-edit-tools --strict` —— `Change 'add-onepiece-search-and-edit-tools' is valid`。

- [x] 9.6 `openspec validate --specs --strict` —— 85 passed, 0 failed（85 items）。

- [x] 9.7 `npm run lint`（本条目原始 tasks.md 草稿未列，Task 10 按 task-10-brief.md Step 1 的实际执行清单补齐）—— `eslint .` 干净，无错误无警告。

- [x] 9.8 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`（同上，原始草稿未列，AGENTS.md「校验命令」隐含要求，Task 10 按 orchestrator 指示一并执行）—— 退出码 0，无差异输出。

### 跨任务复盘：反复出现的"伪阳性测试"问题

以下六次独立的审查/自查里，都发现了同一类问题——**某条测试在它所声称覆盖的护栏被删除/破坏之后仍然全绿**，因为要么是另一个不相关的护栏在同一夹具上顶替报了错，要么断言本身太弱（只查子串/`is_error`，不查具体数值或具体错误原因），要么测的根本不是生产代码真正调用的那个函数。全部六处都已修复，但记录下来是因为这类问题在这条 9 任务链里出现的频率高到值得作为一条通用教训：

1. **Task 4（grep）mutation-testing 复查**：`an_absurdly_large_context_does_not_panic_and_is_clamped` 的夹具把匹配放在第 0 行，`MAX_CONTEXT_LINES` 夹紧逻辑被删除后 `0 + usize::MAX` 依然不会溢出，测试原样通过。
2. **Task 5（edit）实现方自查**：`an_empty_old_string_is_rejected` 的护栏被禁用后，空字符串在 `text.matches("").count()` 下会数出等于文本长度的匹配次数，落进"匹配 >1 次"分支同样报错，`is_error` 断言分辨不出两者。
3. **Task 5（edit）第二轮外部审查**：`multiple_matches_are_rejected_and_the_count_is_reported`/`replace_all_replaces_every_match_and_reports_the_count` 只用 `contains('3')`/`contains('2')` 断言次数，`{occurrences}` 被改写成 `occurrences + 10` 后仍然通过（"13"含"3"，"12"含"2"）。
4. **Task 6（file 读取边界）审查**：单行字符上限的 `line.chars().count() > MAX_READ_LINE_CHARS` 被改成 `>=` 后，26 条既有测试全部仍然通过，因为没有一条测试用"恰好等于上限"的边界夹具。
5. **Task 7（路由/信任）审查**：`search_tools_do_not_require_approval` 测的是 `risk_tier_for`，生产代码真正调用的是叠加了信任标志的 `requires_approval`；把后者改到对 `grep`/`glob` 总是要求审批，前者仍然全绿。
6. **Task 8（目录开关）审查**：`file` 工具的 schema 从未有过"完整参数面"测试；把 `content` 属性删掉、`required` 弱化后，29 条既有测试全部仍然通过。

**结论**：本变更里几乎每一类"结果正确性由一个数值/名单/具体错误信息定义"的护栏，第一版测试都倾向于只断言"有没有报错"或"字符串里有没有某个词"，而不是断言具体的值或具体的原因；只有在审查方主动做 mutation-testing（临时禁用/破坏被测护栏，观察测试是否真的变红）之后，这类伪阳性才会暴露。后续新增工具类测试时，应当默认对"数量/边界/唯一性"类护栏做一次 mutation 验证，而不是等审查发现。

**综合结论（Task 10）**：openspec 两条 `validate --strict` 均通过、前端 lint/test/build 三项均通过、Rust `--lib` 测试与两种 clippy 调用均通过；**但 `cargo test --manifest-path src-tauri/Cargo.toml` 的完整调用（含 `tests/architecture.rs`）未能全绿**，见 §9.1。本变更尚不满足"全部通过"的归档前提，`openspec archive add-onepiece-search-and-edit-tools` 不应在此状态下执行。
