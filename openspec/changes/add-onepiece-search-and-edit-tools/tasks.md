## 1. 共享依赖与受限遍历（walk.rs）

- [ ] 1.1 在 `src-tauri/Cargo.toml` 新增 `regex`、`ignore`、`globset` 直接依赖
- [ ] 1.2 新增 `walk.rs`：`BoundedFilesystem` 边界 + `.gitignore`/`.ignore` 过滤 + 跳过符号链接 + 取消信号 + 结果上限的共享遍历实现
- [ ] 1.3 单元测试覆盖遍历的忽略规则、符号链接跳过、取消、上限行为

## 2. `glob` 工具

- [ ] 2.1 新增 `glob_tool.rs`：按文件名模式匹配，复用 `walk.rs`
- [ ] 2.2 单元测试覆盖匹配与忽略规则行为

## 3. `grep` 工具

- [ ] 3.1 新增 `grep_tool.rs`：`pattern`/`glob`/`path`/`output_mode`/`context`/`case_insensitive`/`head_limit` 参数与三种 `output_mode`
- [ ] 3.2 接入 10MB 输入上限（静默跳过超限文件）与二进制内容跳过
- [ ] 3.3 单元测试覆盖三种 `output_mode`、`.gitignore` 跳过、超限文件跳过

## 4. `edit` 工具

- [ ] 4.1 新增 `edit_tool.rs`：`old_string`/`new_string`/`replace_all` 唯一匹配语义
- [ ] 4.2 接入 10MB 输入上限（报错而非静默跳过）
- [ ] 4.3 单元测试覆盖 0/1/多匹配三分支与超限文件报错

## 5. `file` read 边界

- [ ] 5.1 `file_tool.rs` 的 read 操作增加 `offset`/`limit` 分页与行号前缀
- [ ] 5.2 接入行数/单行字符/总字节三档硬上限（`limit` 只能调低不能调高）与 10MB 前置大小检查（报错）
- [ ] 5.3 二进制内容拒绝（NUL 字节判定，返回明确原因而非 UTF-8 解码错误）
- [ ] 5.4 单元测试覆盖分页、行号、三档上限、二进制拒绝、超限文件报错

## 6. 风险分级、信任白名单与工具路由

- [ ] 6.1 `risk_tier_for()`：`grep`/`glob` 归为 `AutoApprove`，`edit` 归为 `RequiresApproval`
- [ ] 6.2 `requires_approval()` 信任白名单加入 `edit`
- [ ] 6.3 `execute_tool_call()` 路由新增 `grep`/`glob`/`edit` 三个分支
- [ ] 6.4 `execute_tool_call()` 的 plan mode 硬拒逻辑新增 `edit`
- [ ] 6.5 单元测试覆盖风险分级、信任白名单、plan mode 硬拒（含模型主动请求 `edit` 的场景）

## 7. 工具目录开关

- [ ] 7.1 `tool_catalog()` 从 3 个工具扩展到 6 个（`shell`/`file`/`remember`/`grep`/`glob`/`edit`）
- [ ] 7.2 `plan_mode_tool_catalog()` 从 2 个扩展到 4 个（+`grep`/`glob`）
- [ ] 7.3 更新契约测试 `catalog_declares_exactly_shell_file_and_remember_tools`（`catalog.len()` 断言随之变化）

## 8. Web mock grep 示例

- [ ] 8.1 `web-agent-client.ts` 的模拟工具调用序列中补一个 `grep` 调用示例，与桌面能力保持演示保真度

## 9. Verification

- [ ] 9.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 9.2 `cargo clippy --manifest-path src-tauri/Cargo.toml`
- [ ] 9.3 `npm run test`
- [ ] 9.4 `npm run build`
- [ ] 9.5 `openspec validate add-onepiece-search-and-edit-tools --strict`
- [ ] 9.6 `openspec validate --specs --strict`
