# Verification results

宿主：Linux x86_64（Linux 7.0.0-29-generic），worktree `feat/cli-supplement`，叠加在 `extend-cli-providers-with-acp` 与 `harden-sqlite-write-transactions` 的未提交改动之上。执行日期 2026-09-07。Windows / macOS 全部 NOT RUN。

## 命令

| 命令 | 结果 | 退出码 | 说明 |
| --- | --- | --- | --- |
| `openspec validate add-qwen-iflow-config-profiles --strict` | PASSED | 0 | 立项时（改代码前）与实施后各一次 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED | 0 | |
| `cargo check --workspace` | PASSED | 0 | |
| `cargo clippy --workspace --all-targets -- -D warnings`（`-j 3`） | PASSED | 0 | |
| `npm run native:panic:check` | PASSED | 0 | |
| `cargo test --lib -- contexts::tooling::cli_config contexts::tooling::cli::infrastructure::native_config_reader` | PASSED | 0 | 76 passed（首轮 1 failed：iFlow 发现候选的 provider 名按端点主机名生成为 `api.deepseek.com`，断言改为实际规则） |
| `cargo test --lib -- contexts::tooling contexts::sessions` | PASSED | 0 | 1767 passed |
| `cargo test --test architecture` | PASSED | 0 | 63 passed；首轮 `agent_runtime/infrastructure` 聚合行数因 ACP live 测试超预算，按实测 73,112 上调并记归属（生产预算不变） |
| `npm run lint:ci` | PASSED | 0 | |
| `npm run test` | PASSED | 0 | 475 files / 2938 tests |
| `npm run build` | PASSED | 0 | |
| `npm run architecture:check` | PASSED（修正后） | 0 | 首轮 `src/services` 聚合行数超 4 行（Web/mock 校验规则），按实测 27,489 上调并写明理由后通过 |
| `npm run contracts:check` | PASSED | 0 | |
| `npm run test:coverage` | PASSED | 0 | 2938 passed，覆盖率门槛通过 |
| `npm run docs:check` | PASSED | 0 | |
| `openspec validate --specs --strict` | PASSED | 0 | |
| `npx playwright test tests/e2e/agent-global-config.spec.ts` | PASSED | 0 | 4 passed（含新增的 Qwen Code / iFlow DeepSeek 预设创建并全局应用用例） |
| `npx playwright test --shard=N/6`（顺序六次） | PASSED（修正后） | 0 | 43 + 41 + 39 + 45 + 38 + 40 = 246 passed；分片 3 首轮 1 failed：`onepiece-agent.spec.ts` 的 Agent 配置选择器期望仍是六项，加入 Qwen Code / iFlow CLI 后单独复跑通过（见下行） |
| `npx playwright test tests/e2e/onepiece-agent.spec.ts`（复跑） | PASSED | 0 | 3 passed |

## 桌面层（Linux）

| 层 | 结果 | 说明 |
| --- | --- | --- |
| `npm run test:desktop:build` | PASSED | 2026-09-07 07:0x UTC |
| desktop-smoke（首轮） | FAILED → 定位 | `2026-09-07T07-16-07-718Z-a53f6dab`：24/25，仅 `ui-agent-configuration` 的选择器数量断言仍是 6（实际 8：七家 CLI + OnePiece），其余含 `domain-cli-tooling` 七家配置路径全部通过；修正断言后复跑见下行 |
| desktop-smoke（复跑） | PASSED（25/25 spec，0 次重试，8 min 38 s） | `2026-09-07T07-26-03-153Z-5adcb171`。 `domain-cli-tooling` 覆盖七家的状态、路径（Qwen 双文件）、profile 列表、发现与密钥校验；写路径按该层规则不执行 |

Windows / macOS：NOT RUN。
