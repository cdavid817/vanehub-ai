# P0 — Release Signing + Auto Update

## 1. 目标

把当前可发布但允许 unsigned preview 的交付流程升级为：

- Windows 可验证签名；
- macOS Developer ID + notarization；
- Linux integrity/signature policy；
- Tauri updater；
- 更新签名验证；
- stable / prerelease channel；
- 不泄露 release credentials。

本需求必须扩展现有 `desktop-release-delivery`，不能另起一套 release pipeline。

## 2. 建议 OpenSpec change

`add-signed-release-auto-update`

必须先调研：

- `desktop-release-delivery`
- `native-app-packaging`
- `desktop-runtime-verification`
- `native-build-optimization`
- active `publish-verifiable-checksums`
- `docs/...release signing...`
- `.github/workflows/` release workflow
- Tauri v2 updater 当前配置/依赖情况

## 3. Windows Signing

要求：

- release build 支持正式 code signing；
- credential 只来自 protected GitHub environment secrets；
- pull_request workflow 不获取签名 secret；
- signing 缺失时 rehearsal 可按现有 spec 明确产生 unsigned artifact；
- stable public release policy 可在 spec 中决定是否强制签名。

必须提供验证步骤：

```text
artifact -> signature verification -> signer identity -> timestamp
```

不要把证书/private key 写进 repo。

## 4. macOS Signing / Notarization

完整链路：

```text
build
 -> codesign
 -> verify
 -> notarize
 -> staple
 -> verify stapled ticket
 -> publish
```

支持 x64/arm64 当前 release target。

notarization 失败时不得发布“看似签名完整”的 stable artifact。

## 5. Linux Integrity

Linux 至少保留/加强：

- SHA256；
- SBOM；
- attestations；
- 如项目决定加入 detached GPG/minisign，必须在 OpenSpec 明确 key rotation 和 verification docs。

不要误称 checksum 为 code signing。

## 6. Auto Update

采用 Tauri v2 官方 updater 能力（以当前仓库依赖版本为准）。

功能：

- 手动 `Check for updates`；
- 可配置自动检查；
- stable/pre-release channel policy；
- 显示当前版本、新版本；
- release notes；
- download progress；
- signature verification；
- apply/restart；
- failure recovery。

更新检查属于长运行操作，不能阻塞 UI。

## 7. Update Security

- updater metadata 必须签名；
- 客户端内只存 public verification key；
- 私钥仅 release environment；
- 版本比较防 downgrade（除非用户进入显式开发/测试流程）；
- 更新 URL/metadata 不允许被普通 runtime config 静默劫持；
- TLS error 不得自动忽略。

## 8. UI

Settings / About 增加 Update 卡片：

```text
Current: 0.x.x
Channel: Stable / Preview
Last checked: ...
[Check for updates]

Update available:
Version ...
Release notes...
[Download and install]
```

要求：

- i18n 全 locale parity；
- futuristic/minimal；
- loading/progress/error/ready/restart states；
- 下载时已有页面数据不被 blank screen 替换。

## 9. Release Rehearsal

CI 必须有不使用真实 production private credentials 的 rehearsal。

可以验证：

- config；
- artifact collection；
- updater manifest/schema；
- signing steps 是否在有 mock/test credentials 时可执行；
- unsigned branch behavior。

真实 signing secret 只在 protected release job 使用。

## 10. 验收标准

1. 已签 Windows artifact 可以验证 publisher。
2. 已签/公证 macOS artifact 通过 codesign/notary verification。
3. Linux artifact 保留 checksum/SBOM/attestation。
4. 客户端能检测新版本并验证 updater signature。
5. tampered update 被拒绝。
6. prerelease 不错误升级 stable 用户（除非 channel policy 允许）。
7. PR workflow 看不到 signing secrets。
8. manual release rehearsal 不发布 production release。
9. release notes 正确区分 signed/unsigned。

## 11. 测试

- version/update manifest unit tests；
- channel comparison；
- signature verification negative tests；
- updater service Web/mock；
- Playwright settings update states；
- Desktop test 尽可能使用本地/mock update server 验证 update check；
- GitHub Actions matrix 对每个平台分别报告签名/公证 `PASSED/BLOCKED/NOT RUN`；
- 不允许为了 CI 测试把生产 signing key 放进 fixture。


## Codex + OpenSpec 强制执行规则

本需求不是让 Codex 直接开始改代码。执行时必须遵循以下顺序：

1. 先读取仓库根目录 `AGENTS.md`、`openspec/project.md`。
2. 检索 `openspec/specs/` 中与本需求相关的已确认主规范。
3. 检索 `openspec/changes/` 中未归档 change，确认是否已有重叠/冲突工作。
4. 如已有同能力 active change：
   - 优先扩展或完成现有 change；
   - 禁止创建重复的平行架构；
   - 如必须拆分，明确依赖关系和边界。
5. 如无现有 change，按仓库既有 OpenSpec 结构创建 proposal/design/tasks/spec delta。
6. 在改业务代码前执行：
   `openspec validate <change-name> --strict`
7. 按 `tasks.md` 顺序实施，不允许跳过验证任务。
8. 完成后执行所有适用的功能、自动化、UI、视觉、代码规范、性能与桌面运行时验证。
9. 所有验收通过后再归档：
   `openspec archive <change-name>`
10. 归档后执行：
   `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`
11. 一个需求完整结束、归档并提交后，才开始下一个需求。

如果当前 Codex 环境提供 `/opsx:new`、`/opsx:apply` 等 OpenSpec Skill，可使用它们创建/应用 change；但最终目录结构、验证与归档必须符合本仓库 `AGENTS.md`。

### 不允许做的事情

- 不允许为了通过测试而删除、降低或绕过既有测试/CI 门槛。
- 不允许使用 `git commit --no-verify` 或修改 hooks 来绕过规则。
- 不允许新增架构 allowlist 来掩盖新违规。
- 不允许 React 组件直接调用 Tauri `invoke()`。
- 不允许为本需求另建与现有 bounded context 平行的重复领域模型。
- 不允许在没有 OpenSpec proposal 的情况下直接实现架构/功能变更。
- 不允许顺手实现后续需求；发现后续依赖只记录，不提前扩大 scope。

## 全局 Definition of Done

任何涉及代码的 change，至少必须跑通仓库 `AGENTS.md` 当前要求的基础校验：

```bash
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
openspec validate <change-name> --strict
```

按变更类型追加：

- 前端覆盖率：`npm run test:coverage`
- Contract：`npm run contracts:check`
- UI 行为：`npx playwright test`
- 桌面/Tauri/IPC：`npm run desktop:unit:test`、`npm run test:desktop`
- UI 视觉：对受影响页面至少覆盖 `futuristic` / `minimal` 两种样式、桌面宽度和窄宽度；优先使用现有 Playwright screenshot/visual 机制，如不存在则在本 change 中建立最小、稳定、可重复的视觉回归方案。
- 性能敏感路径：必须给出可重复的 benchmark/measurement evidence，CI 中优先使用确定性预算和结构性断言，避免共享 runner 上脆弱的固定毫秒断言。
- Windows/macOS/Linux 原生验证：只报告实际运行过的平台；统一使用 `PASSED` / `FAILED` / `BLOCKED` / `NOT RUN`。

### 完成报告必须包含

- OpenSpec change 名称
- 新增/修改的 capability
- 关键架构决策
- 主要文件
- 数据迁移（如有）
- 测试结果
- UI/视觉验证结果（如适用）
- 性能结果（如适用）
- Desktop Smoke 各平台状态（如适用）
- 已知限制和后续依赖
