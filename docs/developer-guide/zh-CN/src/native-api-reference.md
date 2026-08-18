# Native API 参考

native 参考由 Rustdoc 生成,排除了依赖并包含私有项:

```powershell
cargo doc --manifest-path src-tauri/Cargo.toml --no-deps --document-private-items
```

[打开组装好的 native API 参考](../../api/vanehub_ai_lib/index.html)。

该参考向贡献者公开内部的所有权与契约文档。它不会把 crate 私有项变成受支持的外部库 API,也不会为实现而扩大可见性以方便文档化。

所选的文档边界清单位于 `docs/developer-guide/native-boundaries.json`。
