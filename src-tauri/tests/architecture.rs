use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemUse, Lit, UseTree};

#[test]
fn distributable_release_profile_stays_optimized() {
    // Reads the workspace root, not `src-tauri/Cargo.toml`, because that is where Cargo takes the
    // profile from. A profile declared in a non-root member is ignored with only a warning, so this
    // test passed for as long as the settings were in the member and doing nothing — asserting on
    // manifest text is only meaningful when the text is the one Cargo actually resolves.
    let manifest_path = project_root().join("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path).expect("read workspace manifest");
    let document = manifest
        .parse::<toml::Table>()
        .expect("parse workspace manifest");
    let release = document
        .get("profile")
        .and_then(toml::Value::as_table)
        .and_then(|profiles| profiles.get("release"))
        .and_then(toml::Value::as_table)
        .expect("release profile at the workspace root");

    // The member must not carry one: Cargo would ignore it, so a future edit there would look
    // effective and be silently discarded.
    let member = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read native manifest");
    assert!(
        !member
            .parse::<toml::Table>()
            .expect("parse native manifest")
            .contains_key("profile"),
        "src-tauri/Cargo.toml declares a profile, which Cargo ignores in a non-root workspace member"
    );

    assert_eq!(
        release.get("opt-level").and_then(toml::Value::as_integer),
        Some(3)
    );
    assert_eq!(
        release.get("lto").and_then(toml::Value::as_str),
        Some("thin")
    );
    assert_eq!(
        release
            .get("codegen-units")
            .and_then(toml::Value::as_integer),
        Some(1)
    );
    assert_eq!(
        release.get("strip").and_then(toml::Value::as_str),
        Some("debuginfo")
    );
    assert!(
        release_debug_information_is_disabled(release.get("debug")),
        "release builds must not carry debug information"
    );
    assert_ne!(
        release
            .get("debug-assertions")
            .and_then(toml::Value::as_bool),
        Some(true),
        "release builds must not enable debug assertions"
    );
}

fn release_debug_information_is_disabled(debug: Option<&toml::Value>) -> bool {
    match debug {
        None | Some(toml::Value::Boolean(false)) | Some(toml::Value::Integer(0)) => true,
        Some(toml::Value::String(level)) => level == "none",
        _ => false,
    }
}

/// Whether a native source path holds test code rather than production code.
///
/// A test module takes two shapes in this tree: a `tests.rs`/`*_tests.rs` file, or — once it
/// grows enough to be split by subject — a `tests/` directory of sibling modules. Rules that
/// matched only the file name silently stopped recognizing the second shape, so test code that
/// had been exempt since it was written started tripping production rules the moment it moved
/// into a directory. `relocate-heavyweight-inline-tests` hit exactly that on the provider-neutral
/// rule. One predicate, used by every rule that needs it, is what keeps the three copies from
/// drifting again.
fn is_test_source(relative: &str) -> bool {
    let normalized = relative.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or_default();
    file_name == "tests.rs"
        || file_name.ends_with("_tests.rs")
        || normalized.starts_with("tests/")
        || normalized.contains("/tests/")
}

#[test]
fn test_sources_are_recognized_as_files_and_as_directories() {
    for relative in [
        "contexts/sessions/infrastructure/tests.rs",
        "contexts/agent_runtime/application/loop_control_tests.rs",
        "contexts/agent_runtime/application/tests/message_dispatch.rs",
        "contexts/sessions/infrastructure/tests/recovery.rs",
        "tests/architecture.rs",
    ] {
        assert!(
            is_test_source(relative),
            "{relative} should read as test code"
        );
    }
    // Windows separators reach these rules from `rust_files`, so the predicate normalizes first.
    assert!(is_test_source(
        r"contexts\agent_runtime\application\tests\onepiece_provider.rs"
    ));
    for relative in [
        "contexts/sessions/infrastructure/sqlite_repository.rs",
        "contexts/agent_runtime/application/service.rs",
        "contexts/agent_runtime/application/contest.rs",
    ] {
        assert!(
            !is_test_source(relative),
            "{relative} is production code and must stay covered"
        );
    }
}

#[test]
fn provider_neutral_layers_do_not_select_concrete_cli_providers() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let roots = [
        source_root.join("contexts/sessions/domain"),
        source_root.join("contexts/sessions/application"),
        source_root.join("contexts/agent_runtime/application"),
    ];
    let provider_ids = [
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "opencode",
        "antigravity-cli",
    ];
    let mut violations = Vec::new();

    for root in roots {
        for path in rust_files(&root).expect("enumerate provider-neutral sources") {
            let relative = path
                .strip_prefix(&source_root)
                .expect("relative source path")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            if is_test_source(&relative) {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read provider-neutral source");
            let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
            if production.contains("infrastructure::providers") {
                violations.push(format!(
                    "{relative}: imports concrete provider infrastructure"
                ));
            }
            for provider_id in provider_ids {
                if production.contains(&format!("\"{provider_id}\"")) {
                    violations.push(format!(
                        "{relative}: branches on built-in provider id {provider_id}"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "provider-neutral layers must resolve behavior through contracts:\n{}",
        violations.join("\n")
    );
}

/// Ratchet for the native runtime read cutover: every real managed-CLI launch resolves its
/// user-profile argv through `tooling::api`. Production code in `agent_runtime` and `sessions` may
/// not reach back into the CLI-parameter subdomain's private modules, nor re-acquire the legacy
/// reader that the cutover removed. Test sources are exempt: the dual-read suites deliberately seed
/// `cli_parameter_settings` rows and transcribe the pre-cutover renderer to prove equivalence.
#[test]
fn cli_parameter_consumers_only_reach_the_published_tooling_api() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let roots = [
        source_root.join("contexts/agent_runtime"),
        source_root.join("contexts/sessions"),
    ];
    // `tooling::api` is the only permitted path into the subdomain. Everything else here is either
    // a private module of it or a symbol of the launch reader the cutover deleted.
    let forbidden = [
        (
            "crate::contexts::tooling::cli_parameters::",
            "imports a private CLI-parameter module",
        ),
        (
            "tooling::cli_parameters::domain",
            "imports the CLI-parameter domain",
        ),
        (
            "tooling::cli_parameters::application",
            "imports the CLI-parameter application layer",
        ),
        (
            "tooling::cli_parameters::infrastructure",
            "imports CLI-parameter persistence",
        ),
        (
            "cli_parameter_settings",
            "reads the CLI-parameter table directly",
        ),
        ("preview_args", "calls the removed legacy renderer"),
        (
            "load_selections",
            "calls the removed legacy selection reader",
        ),
        (
            "normalize_selections",
            "calls the removed legacy normalizer",
        ),
    ];
    let mut violations = Vec::new();

    for root in roots {
        for path in rust_files(&root).expect("enumerate CLI-parameter consumer sources") {
            let relative = path
                .strip_prefix(&source_root)
                .expect("relative source path")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            if is_test_source(&relative) {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read CLI-parameter consumer source");
            let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
            for (needle, reason) in forbidden {
                if production.contains(needle) {
                    violations.push(format!("{relative}: {reason} (`{needle}`)"));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "CLI parameters must be consumed through contexts::tooling::api:\n{}",
        violations.join("\n")
    );
}

#[test]
fn token_accounting_keeps_parsing_policy_storage_and_ui_at_their_boundaries() {
    let native_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let application = fs::read_to_string(
        native_root.join("contexts/sessions/application/usage_accounting_ports.rs"),
    )
    .expect("read token accounting ports");
    let storage = fs::read_to_string(
        native_root.join("contexts/sessions/infrastructure/usage_accounting.rs"),
    )
    .expect("read token accounting storage");
    let runtime_infrastructure = native_root.join("contexts/agent_runtime/infrastructure");

    assert!(application.contains("trait TokenAccountingPort"));
    assert!(application.contains("trait TokenAccountingQueryPort"));
    assert!(!application.contains("rusqlite"));
    assert!(storage.contains("impl TokenAccountingRepository for SqliteSessionsRepository"));
    assert!(storage.contains("rusqlite"));
    for parser in [
        "anthropic_provider.rs",
        "openai_compatible_provider.rs",
        "terminal_usage_ledger.rs",
    ] {
        assert!(
            runtime_infrastructure.join(parser).is_file(),
            "{parser} must remain infrastructure"
        );
    }
}

#[test]
fn accounting_invocation_contract_carries_the_complete_correlation_snapshot() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/contexts/sessions/application/usage_accounting.rs"),
    )
    .expect("read accounting contract");
    for field in [
        "generation_id",
        "run_id",
        "operation_id",
        "session_id",
        "message_id",
        "agent_id",
        "provider_id",
        "profile_id",
        "endpoint_id",
        "model_id",
        "interaction_kind",
        "purpose",
        "request_sequence",
        "attempt",
    ] {
        assert!(
            source.contains(&format!("pub(crate) {field}:")),
            "missing invocation field {field}"
        );
    }
}

#[test]
fn release_profile_guard_rejects_every_enabled_debug_information_form() {
    let enabled_debug_values = [
        toml::Value::Boolean(true),
        toml::Value::Integer(1),
        toml::Value::Integer(2),
        toml::Value::String("line-directives-only".to_string()),
        toml::Value::String("line-tables-only".to_string()),
        toml::Value::String("limited".to_string()),
        toml::Value::String("full".to_string()),
    ];

    for debug in &enabled_debug_values {
        assert!(
            !release_debug_information_is_disabled(Some(debug)),
            "expected release debug value {debug:?} to be rejected"
        );
    }

    assert!(release_debug_information_is_disabled(None));
    for debug in [
        toml::Value::Boolean(false),
        toml::Value::Integer(0),
        toml::Value::String("none".to_string()),
    ] {
        assert!(release_debug_information_is_disabled(Some(&debug)));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Domain,
    Application,
    Infrastructure,
    Command,
}

#[derive(Debug)]
struct SourceScope {
    context: String,
    layer: Layer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Violation {
    line: usize,
    dependency: String,
    rule_id: &'static str,
    rule: &'static str,
    repair: &'static str,
}

struct DependencyVisitor<'a> {
    scope: &'a SourceScope,
    violations: BTreeSet<Violation>,
}

impl DependencyVisitor<'_> {
    fn inspect(&mut self, segments: &[String], line: usize) {
        if segments.is_empty() {
            return;
        }
        let dependency = segments.join("::");
        if matches!(self.scope.layer, Layer::Infrastructure | Layer::Command) {
            if imports_cross_context_concrete_persistence(self.scope, segments) {
                self.violations.insert(Violation {
                    line,
                    dependency,
                    rule_id: "ARCH-NATIVE-003",
                    rule: "outer adapters cannot depend on another context's concrete persistence",
                    repair: "depend on a port published by the owning context api",
                });
            }
            return;
        }
        if is_forbidden_technology(segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule_id: "ARCH-NATIVE-001",
                rule: "domain/application code cannot depend on concrete I/O or runtime frameworks",
                repair: "depend on a domain/application port and assemble its adapter in bootstrap",
            });
            return;
        }
        if is_forbidden_outer_layer(self.scope, segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule_id: "ARCH-NATIVE-001",
                rule: "dependencies must point inward from adapters to application and domain",
                repair: "move the dependency behind the inward-facing application boundary",
            });
            return;
        }
        if imports_private_cross_context_module(self.scope, segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule_id: "ARCH-NATIVE-002",
                rule: "cross-context access must use the owning context api module",
                repair: "import the owning context api, an explicit contract, or an event",
            });
        }
    }
}

impl<'ast> Visit<'ast> for DependencyVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !is_test_only(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !is_test_only(&node.attrs) {
            syn::visit::visit_item_fn(self, node);
        }
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        let mut imports = Vec::new();
        flatten_use_tree(&node.tree, Vec::new(), &mut imports);
        let line = node.span().start().line;
        for segments in imports {
            self.inspect(&segments, line);
        }
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        let segments = node
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        self.inspect(&segments, node.span().start().line);
        syn::visit::visit_path(self, node);
    }
}

fn flatten_use_tree(tree: &UseTree, prefix: Vec<String>, imports: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Path(path) => {
            let mut next = prefix;
            next.push(path.ident.to_string());
            flatten_use_tree(&path.tree, next, imports);
        }
        UseTree::Name(name) => {
            let mut path = prefix;
            path.push(name.ident.to_string());
            imports.push(path);
        }
        UseTree::Rename(rename) => {
            let mut path = prefix;
            path.push(rename.ident.to_string());
            imports.push(path);
        }
        UseTree::Glob(_) => imports.push(prefix),
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix.clone(), imports);
            }
        }
    }
}

fn is_forbidden_technology(segments: &[String]) -> bool {
    let root = segments.first().map(String::as_str).unwrap_or_default();
    if matches!(
        root,
        "tauri" | "rusqlite" | "reqwest" | "rmcp" | "keyring" | "portable_pty"
    ) {
        return true;
    }
    matches!(
        segments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["std", "fs", ..]
            | ["std", "net", ..]
            | ["std", "process", ..]
            | ["tokio", "fs", ..]
            | ["tokio", "net", ..]
            | ["tokio", "process", ..]
    )
}

fn is_forbidden_outer_layer(scope: &SourceScope, segments: &[String]) -> bool {
    let path = segments.iter().map(String::as_str).collect::<Vec<_>>();
    if matches!(
        path.as_slice(),
        ["crate", "platform", ..]
            | ["crate", "commands", ..]
            | ["crate", "bootstrap", ..]
            | ["crate", "logging", ..]
            | ["crate", "tasks", ..]
    ) {
        return true;
    }
    if path.len() >= 5 && path[0] == "crate" && path[1] == "contexts" && path[2] == scope.context {
        return match scope.layer {
            Layer::Domain => matches!(path[3], "application" | "infrastructure" | "interfaces"),
            Layer::Application => matches!(path[3], "infrastructure" | "interfaces"),
            Layer::Infrastructure | Layer::Command => false,
        };
    }
    false
}

fn imports_private_cross_context_module(scope: &SourceScope, segments: &[String]) -> bool {
    let path = segments.iter().map(String::as_str).collect::<Vec<_>>();
    path.len() >= 4
        && path[0] == "crate"
        && path[1] == "contexts"
        && path[2] != scope.context
        && path[3] != "api"
        && matches!(scope.layer, Layer::Domain | Layer::Application)
}

fn imports_cross_context_concrete_persistence(scope: &SourceScope, segments: &[String]) -> bool {
    if !matches!(scope.layer, Layer::Infrastructure | Layer::Command) {
        return false;
    }
    let path = segments.iter().map(String::as_str).collect::<Vec<_>>();
    path.len() >= 5
        && path[0] == "crate"
        && path[1] == "contexts"
        && path[2] != scope.context
        && path[3] == "infrastructure"
        && path[4..]
            .iter()
            .any(|segment| segment.ends_with("Repository") || *segment == "NativeDatabase")
}

fn source_scope(relative_path: &Path) -> Option<SourceScope> {
    let parts = relative_path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if let Some(contexts) = parts.iter().position(|part| part == "contexts") {
        let context = parts.get(contexts + 1)?.clone();
        let layer = match parts.get(contexts + 2).map(String::as_str) {
            Some("domain") => Layer::Domain,
            Some("application") => Layer::Application,
            Some("infrastructure") => Layer::Infrastructure,
            _ => return None,
        };
        return Some(SourceScope { context, layer });
    }
    let commands = parts.iter().position(|part| part == "commands")?;
    Some(SourceScope {
        context: parts.get(commands + 1)?.clone(),
        layer: Layer::Command,
    })
}

fn analyze(relative_path: &Path, source: &str) -> Result<Vec<Violation>, String> {
    let Some(scope) = source_scope(relative_path) else {
        return Ok(Vec::new());
    };
    let syntax =
        syn::parse_file(source).map_err(|error| format!("{}: {error}", relative_path.display()))?;
    let mut visitor = DependencyVisitor {
        scope: &scope,
        violations: BTreeSet::new(),
    };
    visitor.visit_file(&syntax);
    Ok(visitor.violations.into_iter().collect())
}

#[derive(Default)]
struct PathDependencyVisitor {
    dependencies: BTreeSet<(usize, Vec<String>)>,
}

impl PathDependencyVisitor {
    fn record(&mut self, segments: Vec<String>, line: usize) {
        if !segments.is_empty() {
            self.dependencies.insert((line, segments));
        }
    }
}

impl<'ast> Visit<'ast> for PathDependencyVisitor {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        let mut imports = Vec::new();
        flatten_use_tree(&node.tree, Vec::new(), &mut imports);
        for segments in imports {
            self.record(segments, node.span().start().line);
        }
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.record(
            node.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect(),
            node.span().start().line,
        );
        syn::visit::visit_path(self, node);
    }
}

fn path_dependencies(source: &str) -> Result<BTreeSet<(usize, Vec<String>)>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut visitor = PathDependencyVisitor::default();
    visitor.visit_file(&syntax);
    Ok(visitor.dependencies)
}

fn runner_boundary_violations(relative_path: &Path, source: &str) -> Result<Vec<String>, String> {
    let mut violations = analyze(relative_path, source)?
        .into_iter()
        .map(|violation| format!("{}: {}", violation.line, violation.dependency))
        .collect::<Vec<_>>();
    for (line, segments) in path_dependencies(source)? {
        let path = segments.iter().map(String::as_str).collect::<Vec<_>>();
        let private_owned_context = context_target(&segments).is_some_and(|(context, layer)| {
            matches!(
                context,
                "ssh_connections" | "permissions" | "sessions" | "operations"
            ) && layer != "api"
        });
        let direct_ssh_transport = path.first() == Some(&"russh");
        if private_owned_context || direct_ssh_transport {
            violations.push(format!("{line}: {}", segments.join("::")));
        }
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in
            fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                visit(&path, files)?;
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("native crate must be inside the project")
        .to_path_buf()
}

fn frontend_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in
            fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                visit(&path, files)?;
            } else if matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("ts" | "tsx")
            ) {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn is_frontend_test(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    name.contains(".test.") || name.contains(".spec.")
}

fn typescript_module_specifiers(source: &str) -> Vec<String> {
    let markers = [
        (" from \"", '"'),
        (" from '", '\''),
        ("import(\"", '"'),
        ("import('", '\''),
    ];
    let mut modules = Vec::new();
    for line in source.lines() {
        for &(marker, quote) in &markers {
            let Some(start) = line.find(marker) else {
                continue;
            };
            let value = &line[start + marker.len()..];
            if let Some(end) = value.find(quote) {
                modules.push(value[..end].to_string());
            }
        }
    }
    modules
}

fn path_segments(path: &str) -> Vec<String> {
    path.split("::").map(str::to_string).collect()
}

fn context_target(segments: &[String]) -> Option<(&str, &str)> {
    (segments.len() >= 4 && segments[0] == "crate" && segments[1] == "contexts")
        .then(|| (segments[2].as_str(), segments[3].as_str()))
}

fn forbidden_lsp_retrieval_context_link(owner: &str, segments: &[String]) -> bool {
    let Some((target, _layer)) = context_target(segments) else {
        return false;
    };
    match owner {
        "agent_runtime" => matches!(target, "code_intelligence" | "retrieval"),
        "code_intelligence" => matches!(target, "agent_runtime" | "retrieval"),
        "retrieval" => matches!(target, "agent_runtime" | "code_intelligence"),
        _ => false,
    }
}

fn forbidden_lsp_retrieval_bridge_link(segments: &[String]) -> bool {
    let Some((target, layer)) = context_target(segments) else {
        return false;
    };
    match target {
        "code_intelligence" | "retrieval" => layer != "api",
        "agent_runtime" => layer != "application",
        _ => false,
    }
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        syn::Type::Reference(reference) => type_name(&reference.elem),
        _ => "anonymous".to_string(),
    }
}

fn root_business_items(source: &str) -> Result<BTreeSet<String>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut keys = BTreeSet::new();
    for item in syntax.items {
        match item {
            Item::Const(item) => {
                keys.insert(format!("const:{}", item.ident));
            }
            Item::Enum(item) => {
                keys.insert(format!("enum:{}", item.ident));
            }
            Item::Fn(item) if item.sig.ident != "run" => {
                keys.insert(format!("fn:{}", item.sig.ident));
            }
            Item::Impl(item) => {
                let owner = type_name(&item.self_ty);
                for member in item.items {
                    match member {
                        ImplItem::Const(member) => {
                            keys.insert(format!("impl:{owner}::const:{}", member.ident));
                        }
                        ImplItem::Fn(member) => {
                            keys.insert(format!("impl:{owner}::fn:{}", member.sig.ident));
                        }
                        ImplItem::Type(member) => {
                            keys.insert(format!("impl:{owner}::type:{}", member.ident));
                        }
                        _ => {}
                    }
                }
            }
            Item::Static(item) => {
                keys.insert(format!("static:{}", item.ident));
            }
            Item::Struct(item) => {
                keys.insert(format!("struct:{}", item.ident));
            }
            Item::Trait(item) => {
                keys.insert(format!("trait:{}", item.ident));
            }
            Item::Type(item) => {
                keys.insert(format!("type:{}", item.ident));
            }
            Item::Union(item) => {
                keys.insert(format!("union:{}", item.ident));
            }
            _ => {}
        }
    }
    Ok(keys)
}

fn is_tauri_command(function: &ItemFn) -> bool {
    function.attrs.iter().any(|attribute| {
        let segments = attribute
            .path()
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        matches!(segments.as_slice(), [tauri, command] if tauri == "tauri" && command == "command")
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CommandMetrics {
    io_decisions: usize,
    control_flow_decisions: usize,
}

struct CommandBodyVisitor {
    metrics: CommandMetrics,
}

impl CommandBodyVisitor {
    fn inspect_path(&mut self, path: &syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if is_forbidden_technology(&segments)
            || matches!(
                segments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .as_slice(),
                ["Command", "new"] | ["Connection", "open"] | ["Connection", "open_in_memory"]
            )
        {
            self.metrics.io_decisions += 1;
        }
    }
}

impl<'ast> Visit<'ast> for CommandBodyVisitor {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.metrics.control_flow_decisions += 1;
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.metrics.control_flow_decisions += 1;
        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.metrics.control_flow_decisions += 1;
        syn::visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.metrics.control_flow_decisions += 1;
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.metrics.control_flow_decisions += 1;
        syn::visit::visit_expr_loop(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if matches!(
            node.method.to_string().as_str(),
            "execute"
                | "execute_batch"
                | "prepare"
                | "query_row"
                | "spawn"
                | "spawn_blocking"
                | "output"
                | "status"
                | "kill"
                | "wait"
        ) {
            self.metrics.io_decisions += 1;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.inspect_path(node);
        syn::visit::visit_path(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast ExprLit) {
        if let Lit::Str(value) = &node.lit {
            let normalized = value.value().to_ascii_uppercase();
            if [
                "SELECT ",
                "INSERT ",
                "UPDATE ",
                "DELETE ",
                "CREATE TABLE",
                "ALTER TABLE",
                "PRAGMA ",
            ]
            .iter()
            .any(|keyword| normalized.contains(keyword))
            {
                self.metrics.io_decisions += 1;
            }
        }
        syn::visit::visit_expr_lit(self, node);
    }
}

fn command_metrics(source: &str) -> Result<Option<CommandMetrics>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let commands = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if is_tauri_command(function) => Some(function),
            _ => None,
        })
        .collect::<Vec<_>>();
    if commands.is_empty() {
        return Ok(None);
    }

    let mut visitor = CommandBodyVisitor {
        metrics: CommandMetrics::default(),
    };
    for command in commands {
        visitor.visit_block(&command.block);
    }
    Ok(Some(visitor.metrics))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeIoUse {
    line: usize,
    kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompositionRootUse {
    line: usize,
    constructor: String,
}

#[derive(Default)]
struct CompositionRootVisitor {
    uses: Vec<CompositionRootUse>,
}

impl<'ast> Visit<'ast> for CompositionRootVisitor {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            if segments.len() >= 2 {
                let owner = &segments[segments.len() - 2];
                let method = &segments[segments.len() - 1];
                if owner == "RuntimeAgentApiAdapter" && method.starts_with("new") {
                    self.uses.push(CompositionRootUse {
                        line: node.span().start().line,
                        constructor: format!("{owner}::{method}"),
                    });
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

#[derive(Default)]
struct RuntimeIoVisitor {
    uses: Vec<RuntimeIoUse>,
}

impl<'ast> Visit<'ast> for RuntimeIoVisitor {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_test_only(&node.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            let tail = segments
                .iter()
                .rev()
                .take(2)
                .map(String::as_str)
                .collect::<Vec<_>>();
            let kind = match tail.as_slice() {
                ["new", "Command"] => Some("direct external-process construction"),
                ["new", "OpenOptions"] => Some("feature-local append-file construction"),
                _ => None,
            };
            if let Some(kind) = kind {
                self.uses.push(RuntimeIoUse {
                    line: node.span().start().line,
                    kind,
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "append"
            && node.args.len() == 1
            && matches!(
                node.args.first(),
                Some(Expr::Lit(ExprLit {
                    lit: Lit::Bool(value),
                    ..
                })) if value.value
            )
        {
            self.uses.push(RuntimeIoUse {
                line: node.span().start().line,
                kind: "feature-local append-file writer",
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn is_test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("test") {
            return true;
        }
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        let mut test = false;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                test = true;
            }
            Ok(())
        });
        test
    })
}

fn runtime_io_uses(source: &str) -> Result<Vec<RuntimeIoUse>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut visitor = RuntimeIoVisitor::default();
    visitor.visit_file(&syntax);
    Ok(visitor.uses)
}

fn composition_root_uses(source: &str) -> Result<Vec<CompositionRootUse>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut visitor = CompositionRootVisitor::default();
    visitor.visit_file(&syntax);
    Ok(visitor.uses)
}

#[test]
fn native_context_dependencies_point_inward() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut messages = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path");
        let source = fs::read_to_string(&path).expect("read native Rust source");
        for violation in analyze(relative, &source).expect("parse native Rust source") {
            messages.push(format!(
                "[{}] {}:{}: {} (`{}`). Repair: {}",
                violation.rule_id,
                relative.display(),
                violation.line,
                violation.rule,
                violation.dependency,
                violation.repair
            ));
        }
    }

    assert!(
        messages.is_empty(),
        "native architecture dependency violations:\n{}",
        messages.join("\n")
    );
}

#[test]
fn detector_reports_framework_and_private_context_dependencies_with_lines() {
    let source = r#"
use rusqlite::Connection;
use crate::contexts::tooling::infrastructure::SqliteToolRepository;

pub fn invalid(_: Connection) {
    let _ = SqliteToolRepository;
}
"#;
    let violations = analyze(
        Path::new("contexts/sessions/application/use_cases.rs"),
        source,
    )
    .expect("analyze fixture");

    assert!(violations.iter().any(|violation| {
        violation.line == 2
            && violation.rule_id == "ARCH-NATIVE-001"
            && violation.dependency.starts_with("rusqlite::Connection")
    }));
    assert!(violations.iter().any(|violation| {
        violation.line == 3
            && violation.rule_id == "ARCH-NATIVE-002"
            && violation
                .dependency
                .starts_with("crate::contexts::tooling::infrastructure")
    }));
}

#[test]
fn detector_allows_published_cross_context_api() {
    let source = "use crate::contexts::operations::api::OperationPublisher;";
    let violations = analyze(
        Path::new("contexts/sessions/application/use_cases.rs"),
        source,
    )
    .expect("analyze fixture");

    assert!(violations.is_empty());
}

#[test]
fn runner_contracts_and_adapters_use_only_published_runtime_boundaries() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let runner_root = source_root.join("contexts/agent_runtime");
    let mut violations = Vec::new();
    for path in rust_files(&runner_root).expect("enumerate Agent runtime") {
        let source = fs::read_to_string(&path).expect("read Agent runtime source");
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if is_test_source(&path.to_string_lossy()) {
            continue;
        }
        if !file_name.contains("runner") && !source.contains("impl AgentRunner") {
            continue;
        }
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative Runner path");
        for violation in runner_boundary_violations(relative, &source).expect("analyze Runner") {
            violations.push(format!("{}:{violation}", relative.display()));
        }
    }
    assert!(
        violations.is_empty(),
        "Runner code must use application ports and published context APIs:\n{}",
        violations.join("\n")
    );
}

#[test]
fn runner_boundary_detector_allows_apis_and_rejects_private_or_concrete_io() {
    let allowed = r#"
use crate::contexts::ssh_connections::api::SshConnectionsApi;
use crate::contexts::permissions::api::PermissionsApi;
fn assemble(_ssh: SshConnectionsApi, _permissions: PermissionsApi) {}
"#;
    assert!(runner_boundary_violations(
        Path::new("contexts/agent_runtime/infrastructure/ssh_runner.rs"),
        allowed
    )
    .expect("allowed fixture")
    .is_empty());

    let private = r#"
use crate::contexts::ssh_connections::infrastructure::SshConnectionPool;
use crate::contexts::permissions::infrastructure::SqlitePermissionRepository;
use russh::client::Handle;
"#;
    let violations = runner_boundary_violations(
        Path::new("contexts/agent_runtime/infrastructure/ssh_runner.rs"),
        private,
    )
    .expect("private fixture");
    assert_eq!(violations.len(), 3);

    let application_io = r#"
use std::process::Command;
use tauri::State;
fn spawn() { let _ = Command::new("agent"); }
"#;
    let violations = runner_boundary_violations(
        Path::new("contexts/agent_runtime/application/runner_bypass.rs"),
        application_io,
    )
    .expect("application fixture");
    assert!(violations.iter().any(|item| item.contains("std::process")));
    assert!(violations.iter().any(|item| item.contains("tauri::State")));
}

#[test]
fn detector_rejects_domain_to_infrastructure_dependency() {
    let source = "use crate::contexts::sessions::infrastructure::SqliteSessionsRepository;";
    let violations = analyze(Path::new("contexts/sessions/domain/session.rs"), source)
        .expect("analyze domain fixture");

    assert!(violations.iter().any(|violation| {
        violation.line == 1
            && violation.rule_id == "ARCH-NATIVE-001"
            && violation.dependency.contains("sessions::infrastructure")
    }));
}

#[test]
fn detector_rejects_cross_context_concrete_persistence_from_outer_adapters() {
    let dependency =
        "use crate::contexts::agent_runtime::infrastructure::SqliteNativeToolRepository;";
    for path in [
        "contexts/cli_delegation/infrastructure/adapter.rs",
        "commands/cli_delegation/start.rs",
    ] {
        let violations = analyze(Path::new(path), dependency).expect("analyze outer adapter");
        assert!(violations.iter().any(|violation| {
            violation.rule
                == "outer adapters cannot depend on another context's concrete persistence"
        }));
    }
}

#[test]
fn detector_allows_outer_adapters_to_use_published_cross_context_ports() {
    let dependency = "use crate::contexts::agent_runtime::api::NativeToolPersistencePort;";
    for path in [
        "contexts/cli_delegation/infrastructure/adapter.rs",
        "commands/cli_delegation/start.rs",
    ] {
        assert!(
            analyze(Path::new(path), dependency)
                .expect("analyze published port")
                .is_empty(),
            "{path}"
        );
    }
}

#[test]
fn code_intelligence_context_exposes_a_layered_public_api_boundary() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let context_root = source_root.join("contexts/code_intelligence");
    let contexts_module =
        fs::read_to_string(source_root.join("contexts/mod.rs")).expect("read contexts module");

    assert!(
        contexts_module.contains("pub(crate) mod code_intelligence;"),
        "contexts/mod.rs must register the code_intelligence bounded context"
    );
    for relative in [
        "mod.rs",
        "api.rs",
        "application/mod.rs",
        "domain/mod.rs",
        "infrastructure/mod.rs",
    ] {
        assert!(
            context_root.join(relative).is_file(),
            "code_intelligence must expose the expected layered boundary: {relative}"
        );
    }

    let mut violations = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        if path.starts_with(&context_root) {
            continue;
        }
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path");
        let source = fs::read_to_string(&path).expect("read native Rust source");
        for (line, segments) in path_dependencies(&source).expect("parse native Rust source") {
            if segments.len() >= 4
                && segments[0] == "crate"
                && segments[1] == "contexts"
                && segments[2] == "code_intelligence"
                && segments[3] != "api"
            {
                violations.push(format!(
                    "{}:{line}: {}",
                    relative.display(),
                    segments.join("::")
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "cross-context code_intelligence access must use its api module:\n{}",
        violations.join("\n")
    );
}

#[test]
fn code_intelligence_never_imports_private_retrieval_layers() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let context_root = source_root.join("contexts/code_intelligence");
    assert!(
        context_root.is_dir(),
        "code_intelligence bounded context must exist before its dependencies can be checked"
    );

    let mut violations = Vec::new();
    for path in rust_files(&context_root).expect("enumerate code_intelligence sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path");
        let source = fs::read_to_string(&path).expect("read code_intelligence source");
        for (line, segments) in path_dependencies(&source).expect("parse code_intelligence source")
        {
            if segments.len() >= 4
                && segments[0] == "crate"
                && segments[1] == "contexts"
                && segments[2] == "retrieval"
                && segments[3] != "api"
            {
                violations.push(format!(
                    "{}:{line}: {}",
                    relative.display(),
                    segments.join("::")
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "code_intelligence must not import retrieval internals:\n{}",
        violations.join("\n")
    );
}

#[test]
fn native_agent_runtime_injects_configured_lsp_responder_into_normal_and_plan_catalogs() {
    let native_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bootstrap = fs::read_to_string(native_root.join("src/bootstrap/agent_runtime.rs"))
        .expect("read agent runtime bootstrap");
    // The behavior tests this guard leans on live in the adapter's child test module, not in
    // the adapter itself (`extract-api-adapter-inline-tests`).
    let adapter = fs::read_to_string(
        native_root.join("src/contexts/agent_runtime/infrastructure/api_process_adapter/tests.rs"),
    )
    .expect("read API process adapter tests");

    for behavior_test in [
        "normal_generation_registers_all_read_only_lsp_tools_when_available",
        "plan_mode_registers_the_same_read_only_lsp_tools_when_available",
        "plan_mode_executes_all_four_read_only_lsp_tools",
    ] {
        assert!(
            adapter.contains(behavior_test),
            "the production wiring guard depends on `{behavior_test}` covering the four read-only catalogs"
        );
    }

    assert!(
        bootstrap.contains(
            "pub(crate) code_intelligence: Arc<dyn AgentCodeIntelligenceResponderPort>"
        ),
        "configured trusted workspaces must supply a concrete code-intelligence responder to the native runtime"
    );
    assert!(
        bootstrap.contains("RuntimeAgentApiAdapter::new_with_code_intelligence("),
        "native bootstrap must use the constructor that accepts the production responder"
    );
    assert!(
        bootstrap.contains("dependencies.code_intelligence,"),
        "native bootstrap must pass its configured responder into the API adapter"
    );
}

#[test]
fn react_components_never_invoke_lsp_commands_directly() {
    let project_root = project_root();
    let frontend_root = project_root.join("src");
    let lsp_commands = [
        "get_lsp_configuration",
        "save_lsp_configuration",
        "list_lsp_workspace_trust",
        "update_lsp_workspace_trust",
        "discover_lsp_servers",
        "test_lsp_server",
        "list_lsp_server_status",
    ];
    let mut inspected = 0;
    let mut violations = Vec::new();
    for path in frontend_files(&frontend_root).expect("enumerate frontend sources") {
        if path.extension().and_then(|value| value.to_str()) != Some("tsx")
            || is_frontend_test(&path)
        {
            continue;
        }
        inspected += 1;
        let source = fs::read_to_string(&path).expect("read React source");
        let relative = path
            .strip_prefix(&project_root)
            .expect("relative React path")
            .to_string_lossy()
            .replace('\\', "/");
        for token in ["@tauri-apps/", "invoke(", "invoke ("] {
            if source.contains(token) {
                violations.push(format!("{relative}: direct native token `{token}`"));
            }
        }
        for command in lsp_commands {
            if source.contains(command) {
                violations.push(format!("{relative}: direct LSP command `{command}`"));
            }
        }
    }

    assert!(
        inspected > 0,
        "no production React components were inspected"
    );
    assert!(
        violations.is_empty(),
        "React must reach LSP through AgentService, never Tauri invoke:\n{}",
        violations.join("\n")
    );
}

#[test]
fn web_lsp_mode_cannot_reach_native_process_or_filesystem_adapters() {
    let services = project_root().join("src/services");
    let web_lsp_path = services.join("web-lsp-client.ts");
    let web_agent_path = services.join("web-agent-client.ts");
    let web_lsp = fs::read_to_string(&web_lsp_path).expect("read Web LSP adapter");
    let web_agent = fs::read_to_string(&web_agent_path).expect("read Web Agent adapter");
    let forbidden = [
        "@tauri-apps/",
        "tauri-agent-client",
        "invoke(",
        "invoke (",
        "node:fs",
        "node:child_process",
        "showOpenFilePicker",
        "showDirectoryPicker",
        "FileSystemHandle",
    ];
    let mut violations = Vec::new();
    for (name, source) in [
        ("web-lsp-client.ts", web_lsp.as_str()),
        ("web-agent-client.ts", web_agent.as_str()),
    ] {
        for token in forbidden {
            if source.contains(token) {
                violations.push(format!("{name}: native capability token `{token}`"));
            }
        }
    }

    let imports = typescript_module_specifiers(&web_lsp);
    let allowed = ["./agent-service", "./lsp-contract", "../types/lsp"];
    for import in &imports {
        if !allowed.contains(&import.as_str()) {
            violations.push(format!(
                "web-lsp-client.ts: unreviewed dependency `{import}`"
            ));
        }
    }
    assert_eq!(
        imports.iter().cloned().collect::<BTreeSet<_>>(),
        allowed.iter().map(|value| (*value).to_string()).collect(),
        "Web LSP import guard is stale"
    );
    assert!(web_agent.contains("import { webLspClient } from \"./web-lsp-client\";"));
    assert!(web_agent.contains("...webLspClient"));
    assert!(
        violations.is_empty(),
        "Web LSP mode must remain an in-memory adapter:\n{}",
        violations.join("\n")
    );
}

#[test]
fn lsp_and_retrieval_communicate_only_through_owned_ports_and_public_apis() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let contexts_root = source_root.join("contexts");
    let mut violations = Vec::new();
    for owner in ["agent_runtime", "code_intelligence", "retrieval"] {
        let owner_root = contexts_root.join(owner);
        for path in rust_files(&owner_root).expect("enumerate bounded context") {
            let source = fs::read_to_string(&path).expect("read bounded-context source");
            let relative = path
                .strip_prefix(&source_root)
                .expect("relative context path");
            for (line, segments) in path_dependencies(&source).expect("parse context source") {
                if forbidden_lsp_retrieval_context_link(owner, &segments) {
                    violations.push(format!(
                        "{}:{line}: {}",
                        relative.display(),
                        segments.join("::")
                    ));
                }
            }
        }
    }

    let bridge_path = source_root.join("bootstrap/code_intelligence.rs");
    let bridge = fs::read_to_string(&bridge_path).expect("read LSP composition root");
    let bridge_dependencies = path_dependencies(&bridge).expect("parse LSP composition root");
    for required in [
        "crate::contexts::agent_runtime::application::AgentWorkspaceMutationPort",
        "crate::contexts::code_intelligence::api::CodeIntelligenceApi",
        "crate::contexts::retrieval::api::CodeIndexApi",
    ] {
        assert!(
            bridge_dependencies
                .iter()
                .any(|(_, segments)| segments.join("::") == required),
            "composition root must retain reviewed boundary `{required}`"
        );
    }
    for (line, segments) in bridge_dependencies {
        if forbidden_lsp_retrieval_bridge_link(&segments) {
            violations.push(format!(
                "bootstrap/code_intelligence.rs:{line}: {}",
                segments.join("::")
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "LSP/retrieval boundaries must use public APIs or Agent-owned ports:\n{}",
        violations.join("\n")
    );
}

#[test]
fn lsp_architecture_detectors_reject_direct_boundary_bypasses() {
    assert!(forbidden_lsp_retrieval_context_link(
        "agent_runtime",
        &path_segments("crate::contexts::code_intelligence::api::CodeIntelligenceApi")
    ));
    assert!(forbidden_lsp_retrieval_context_link(
        "retrieval",
        &path_segments("crate::contexts::code_intelligence::infrastructure::ProcessRegistry")
    ));
    assert!(forbidden_lsp_retrieval_bridge_link(&path_segments(
        "crate::contexts::retrieval::infrastructure::SqliteCodeIndexRepository"
    )));
    assert!(!forbidden_lsp_retrieval_bridge_link(&path_segments(
        "crate::contexts::retrieval::api::CodeIndexApi"
    )));
    assert_eq!(
        typescript_module_specifiers(
            "import { invoke } from \"@tauri-apps/api/core\";\nimport('./native-helper');"
        ),
        ["@tauri-apps/api/core", "./native-helper"]
    );
}

#[test]
fn root_lib_contains_no_business_symbols() {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("read lib.rs");
    let current = root_business_items(&source).expect("parse root business items");
    assert!(
        current.is_empty(),
        "lib.rs contains business symbols:\n{}",
        current.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn migrated_session_code_cannot_return_to_root_or_legacy_modules() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(crate_root.join("src/lib.rs")).expect("read lib.rs");
    let forbidden_declarations = [
        "struct AutomaticArchivalSettings",
        "struct SessionSearchMatch",
        "struct SessionSearchResult",
        "struct SessionCategory",
        "enum UsageStatisticsRange",
        "struct UsageStatistics",
        "struct SessionExportPayload",
        "enum SessionExportFormat",
        "struct SessionExportResult",
        "fn get_automatic_archival_settings_from_conn(",
        "fn insert_chat_message(",
        "fn insert_chat_message_with_references(",
        "fn compose_prompt_with_file_references(",
        "fn list_chat_messages(",
        "fn list_all_chat_messages(",
        "fn build_session_export_payload(",
        "fn serialize_session_export",
        "fn export_file_extension(",
        "fn safe_export_filename(",
        "fn export_session_to_directory(",
        "fn usage_range_start(",
        "fn aggregate_usage_statistics(",
        "fn complete_assistant_message(",
        "fn fail_assistant_message(",
        "fn clear_active_session_if_matches(",
        "fn recover_orphan_session_state(",
        "fn archive_inactive_sessions(",
        "fn search_session_matches(",
        "fn search_sessions_from_conn(",
        "fn load_session_category(",
        "fn create_session_category_in_conn(",
        "fn rename_session_category_in_conn(",
        "fn delete_session_category_in_conn(",
        "fn assign_session_category_in_conn(",
    ];

    for declaration in forbidden_declarations {
        assert!(
            !source.contains(declaration),
            "migrated session declaration returned to lib.rs: {declaration}"
        );
    }
    for legacy_module in ["src/session_configuration.rs", "src/usage.rs"] {
        assert!(
            !crate_root.join(legacy_module).exists(),
            "migrated session module returned: {legacy_module}"
        );
    }
}

#[test]
fn tauri_command_adapters_cannot_gain_io_or_control_flow_decisions() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut over_budget = Vec::new();
    let mut candidates = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read command source");
        let Some(actual) = command_metrics(&source).expect("parse command source") else {
            continue;
        };
        let allowed = CommandMetrics::default();
        candidates.push(format!(
            "{relative}|{}|{}",
            actual.io_decisions, actual.control_flow_decisions
        ));
        if actual.io_decisions > allowed.io_decisions
            || actual.control_flow_decisions > allowed.control_flow_decisions
        {
            over_budget.push(format!(
                "[ARCH-NATIVE-003] {relative}:1: io {}/{}; control flow {}/{}. Repair: map transport only and delegate policy/I/O to an application use case",
                actual.io_decisions,
                allowed.io_decisions,
                actual.control_flow_decisions,
                allowed.control_flow_decisions
            ));
        }
    }

    assert!(
        over_budget.is_empty(),
        "Tauri command adapter decision budgets exceeded:\n{}\n\nCurrent budget candidate:\n{}",
        over_budget.join("\n"),
        candidates.join("\n")
    );
}

#[test]
fn command_thinness_detector_accepts_delegation_and_rejects_io_policy() {
    let compliant = r#"
#[tauri::command]
fn load(service: Service) -> Result<Value, String> { service.load() }
"#;
    let violating = r#"
#[tauri::command]
fn load(connection: Connection) -> Result<Value, String> {
    if ready() { connection.execute("SELECT value FROM settings", []) } else { fallback() }
}
"#;

    assert_eq!(
        command_metrics(compliant).expect("parse compliant command"),
        Some(CommandMetrics::default())
    );
    let metrics = command_metrics(violating)
        .expect("parse violating command")
        .expect("command fixture");
    assert!(metrics.io_decisions > 0);
    assert!(metrics.control_flow_decisions > 0);
}

#[test]
fn runtime_processes_and_append_logs_use_shared_adapters() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read native Rust source");
        for usage in runtime_io_uses(&source).expect("parse native Rust source") {
            let allowed = match usage.kind {
                "direct external-process construction" => relative == "platform/process/mod.rs",
                "feature-local append-file construction" | "feature-local append-file writer" => {
                    relative == "platform/logging.rs" || relative == "platform/private_relay_fs.rs"
                }
                _ => false,
            };
            if !allowed {
                violations.push(format!("[ARCH-NATIVE-004] {relative}:{}: {}. Repair: use the shared platform adapter and assemble it in bootstrap", usage.line, usage.kind));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "runtime I/O bypasses shared platform/operations adapters:\n{}",
        violations.join("\n")
    );
}

#[test]
fn runtime_io_detector_ignores_test_fixtures_but_reports_runtime_bypasses() {
    let source = r#"
fn runtime() {
    let _ = std::process::Command::new("tool");
    let _ = std::fs::OpenOptions::new().append(true);
}

#[cfg(test)]
mod tests {
    fn fixture() {
        let _ = std::process::Command::new("fixture");
    }
}
"#;

    let uses = runtime_io_uses(source).expect("analyze fixture");

    assert_eq!(uses.len(), 3);
    assert!(uses.iter().all(|usage| usage.line <= 4));
}

#[test]
fn composition_root_detector_reports_reviewed_constructors() {
    let source = r#"
fn assemble() {
    let _ = RuntimeAgentApiAdapter::new_without_code_intelligence(dependencies);
}
"#;
    let uses = composition_root_uses(source).expect("analyze fixture");
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].line, 3);
}

#[test]
fn concrete_runtime_dependencies_are_assembled_only_in_bootstrap() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path")
            .to_string_lossy()
            .replace('\\', "/");
        if relative.starts_with("bootstrap/") || is_test_source(&relative) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read native Rust source");
        for usage in composition_root_uses(&source).expect("parse native Rust source") {
            violations.push(format!(
                "[ARCH-NATIVE-005] {relative}:{}: concrete constructor `{}` outside bootstrap. Repair: assemble the concrete dependency in bootstrap",
                usage.line, usage.constructor
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "concrete runtime assembly escaped bootstrap:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_logging_contract_is_not_debug_assertion_gated() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let logging_contract_files = [
        "platform/logging.rs",
        "contexts/operations/infrastructure/unified_logging.rs",
        "contexts/communications/infrastructure/runtime_manager.rs",
        "contexts/desktop/infrastructure/folder_openers.rs",
    ];

    for relative in logging_contract_files {
        let source = fs::read_to_string(source_root.join(relative)).expect("read logging source");
        assert!(
            !source.contains("cfg(debug_assertions)"),
            "production logging contract cannot be debug-assertion gated: {relative}"
        );
    }

    let logging =
        fs::read_to_string(source_root.join("platform/logging.rs")).expect("read log levels");
    for variant in ["Error", "Warn", "Info", "Debug"] {
        assert!(
            logging.contains(&format!("    {variant},")),
            "production log level is missing: {variant}"
        );
    }
}

#[test]
fn communications_completion_wait_stays_event_driven_without_sqlite_polling() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let adapter_path =
        source_root.join("contexts/communications/infrastructure/application_adapters.rs");
    let adapter = fs::read_to_string(adapter_path).expect("read communications Agent adapter");
    assert!(adapter.contains("send_message_with_completion"));
    assert!(adapter.contains("recv_timeout"));
    assert!(!adapter.contains("list_messages"));
    assert!(!source_root
        .join("contexts/communications/infrastructure/session_completion.rs")
        .exists());
}

/// `RetrievalError::Storage`/`Embedding` 的 `Display` 会把 rusqlite 消息和 provider 响应片段拼进
/// 字符串，而设置页把 command 返回的错误串**原样**渲染（`onepiece-retrieval-section.tsx` 的
/// `operationError`）。设计文档 §8.2 规定这类文本既不落盘也不外露，所以凡是碰 `RetrievalApi` 的
/// command 都必须用 `category()` 过一道再跨边界，不能直接 `to_string()` 整个错误。
///
/// 逐个文件盯着改会漏——`save_retrieval_configuration` 就是这么漏的：它一直用 `to_string()`，
/// 而 C1 的修复又给 `save_configuration` 新增了一条 `Storage` 失败路径（`requeue_stale_model`），
/// 把一个原本只在理论上存在的泄漏变成了实际可达的。这个守卫按"是否持有 `RetrievalApi`"筛文件，
/// 所以以后新增的 retrieval command 会自动被覆盖。
#[test]
fn commands_holding_the_retrieval_api_never_return_error_payload_text() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    let mut inspected = Vec::new();
    for path in rust_files(&source_root).expect("enumerate native Rust sources") {
        let relative = path
            .strip_prefix(&source_root)
            .expect("relative source path")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read command source");
        if !holds_retrieval_api_in_a_command(&source).expect("parse command source") {
            continue;
        }
        inspected.push(relative.clone());
        for line in raw_error_display_conversions(&source).expect("parse command source") {
            violations.push(format!("{relative}:{line}"));
        }
    }

    // 检测器自身的护栏：筛选条件一旦失效（比如类型改名），上面的循环会静默地一个文件都不看，
    // 断言就会变成永真。
    assert!(
        inspected.len() >= 4,
        "detector matched too few RetrievalApi command files ({inspected:?}); it is broken"
    );
    assert!(
        violations.is_empty(),
        "these commands hand the raw error Display (rusqlite/provider text) to the settings page \
         instead of `error.category()`:\n{}\n\nInspected:\n{}",
        violations.join("\n"),
        inspected.join("\n")
    );
}

/// 判定依据是"文件里既有 `#[tauri::command]`，代码路径里又出现 `RetrievalApi`"。用代码路径而不是
/// 文本搜索，是为了让 `list_embedding_models` 那种只在文档注释里提到 `retrieval::api`、实际持有
/// `AgentRuntimeApi` 的命令不被误判——它返回的是另一套错误类型，本守卫管不着。
fn holds_retrieval_api_in_a_command(source: &str) -> Result<bool, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;

    struct RetrievalApiVisitor {
        has_command: bool,
        mentions_api: bool,
    }

    impl<'ast> Visit<'ast> for RetrievalApiVisitor {
        fn visit_item_fn(&mut self, node: &'ast ItemFn) {
            if is_tauri_command(node) {
                self.has_command = true;
            }
            syn::visit::visit_item_fn(self, node);
        }

        fn visit_path(&mut self, node: &'ast syn::Path) {
            if node
                .segments
                .iter()
                .any(|segment| segment.ident == "RetrievalApi")
            {
                self.mentions_api = true;
            }
            syn::visit::visit_path(self, node);
        }
    }

    let mut visitor = RetrievalApiVisitor {
        has_command: false,
        mentions_api: false,
    };
    visitor.visit_file(&syntax);
    Ok(visitor.has_command && visitor.mentions_api)
}

/// 只认 `map_err(|error| error.to_string())` 这种把闭包参数**整个**转成字符串的形状。
/// `map_err(|error| error.category().to_string())` 的接收者是方法调用而不是裸路径，
/// `map_err(map_command_error)` 压根不是闭包，两者都不算违规。
fn raw_error_display_conversions(source: &str) -> Result<Vec<usize>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;

    struct MapErrVisitor {
        lines: Vec<usize>,
    }

    impl<'ast> Visit<'ast> for MapErrVisitor {
        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            if node.method == "map_err" {
                if let Some(Expr::Closure(closure)) = node.args.first() {
                    if let Some(syn::Pat::Ident(binding)) = closure.inputs.first() {
                        let mut body = ClosureBodyVisitor {
                            parameter: binding.ident.to_string(),
                            lines: Vec::new(),
                        };
                        body.visit_expr(&closure.body);
                        self.lines.extend(body.lines);
                    }
                }
            }
            syn::visit::visit_expr_method_call(self, node);
        }
    }

    struct ClosureBodyVisitor {
        parameter: String,
        lines: Vec<usize>,
    }

    impl<'ast> Visit<'ast> for ClosureBodyVisitor {
        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            if node.method == "to_string" {
                if let Expr::Path(receiver) = node.receiver.as_ref() {
                    if receiver.path.is_ident(self.parameter.as_str()) {
                        self.lines.push(node.span().start().line);
                    }
                }
            }
            syn::visit::visit_expr_method_call(self, node);
        }
    }

    let mut visitor = MapErrVisitor { lines: Vec::new() };
    visitor.visit_file(&syntax);
    Ok(visitor.lines)
}

/// Every console-subsystem probe the app runs (`where`, `reg`, `node --version`) is spawned from a
/// GUI-subsystem process. Without `CREATE_NO_WINDOW` Windows allocates a console for each one, so
/// startup detection flashes a burst of console windows across the user's desktop.
///
/// `spawn_detached` is deliberately excluded: it passes `DETACHED_PROCESS`, and Windows ignores
/// `CREATE_NO_WINDOW` when that flag is present.
#[test]
fn windows_command_constructors_suppress_console_windows() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("platform")
        .join("process")
        .join("mod.rs");
    let source = fs::read_to_string(&path).expect("read platform process adapter");
    let syntax = syn::parse_file(&source).expect("parse platform process adapter");

    let mut missing = Vec::new();
    for expected in ["std_command", "tokio_command"] {
        let function = syntax
            .items
            .iter()
            .find_map(|item| match item {
                Item::Fn(function) if function.sig.ident == expected => Some(function),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{expected} is declared in platform/process/mod.rs"));

        let mut visitor = ConsoleSuppressionVisitor { found: false };
        visitor.visit_item_fn(function);
        if !visitor.found {
            missing.push(expected);
        }
    }

    assert!(
        missing.is_empty(),
        "these command constructors never suppress the child console window, so every console \
         subsystem child flashes a window: {}",
        missing.join(", ")
    );
}

#[derive(Default)]
struct ConsoleSuppressionVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for ConsoleSuppressionVisitor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(callee) = node.func.as_ref() {
            if callee
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "suppress_console_window")
            {
                self.found = true;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "suppress_console_window" {
            self.found = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Guards the flag value itself: a typo here would silently reintroduce the visible console.
#[test]
#[cfg(windows)]
fn console_suppression_flag_matches_the_windows_constant() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("platform")
            .join("process")
            .join("mod.rs"),
    )
    .expect("read platform process adapter");

    assert!(
        source.contains("0x0800_0000"),
        "CREATE_NO_WINDOW (0x08000000) must be the flag applied by suppress_console_window"
    );
}

/// `CommandExt::creation_flags` replaces the flag word rather than merging into it, so a wrapper
/// that sets flags before spawning silently discards console suppression applied earlier. That is
/// how `CREATE_NO_WINDOW` set in `std_command` stopped reaching every job-contained probe.
///
/// Every call must therefore carry its own suppression, except one that requests
/// `DETACHED_PROCESS` (`0x0000_0008`) — Windows ignores `CREATE_NO_WINDOW` alongside it.
#[test]
fn every_creation_flags_call_keeps_the_child_console_hidden() {
    let process_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("platform")
        .join("process");

    let mut violations = Vec::new();
    let mut inspected = 0usize;
    for path in rust_files(&process_root).expect("enumerate process adapter sources") {
        let source = fs::read_to_string(&path).expect("read process adapter source");
        let relative = path
            .strip_prefix(&process_root)
            .expect("relative source path")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");

        for (index, line) in source.lines().enumerate() {
            if !line.contains(".creation_flags(") {
                continue;
            }
            inspected += 1;
            let detached = line.contains("0x0000_0008") || line.contains("DETACHED_PROCESS");
            let suppressed = line.contains("CREATE_NO_WINDOW");
            if !detached && !suppressed {
                violations.push(format!("{relative}:{}: {}", index + 1, line.trim()));
            }
        }
    }

    assert!(
        inspected > 0,
        "no creation_flags call was found, so this guard is asserting nothing"
    );
    assert!(
        violations.is_empty(),
        "these creation_flags calls overwrite console suppression, so their children flash a \
         window:\n{}",
        violations.join("\n")
    );
}

/// A recorded ceiling for a path that predates the repository's 300-line limit.
///
/// This is a debt marker, not an allowance. Lowering it needs no ceremony; raising it is an
/// explicit edit that states why. `owner` names the change expected to retire the entry, so a
/// failure points at the decomposition work rather than at a bare number.
struct PathBudget {
    path: &'static str,
    budget: usize,
    owner: &'static str,
}

/// An aggregate ceiling for a directory.
///
/// This is what keeps a path budget honest once the file it names becomes a directory module:
/// the path disappears, but the code that replaced it stays bounded — and a "split" that copies
/// instead of moves still trips the gate.
struct SubtreeBudget {
    root: &'static str,
    budget: usize,
    owner: &'static str,
}

const NATIVE_PATH_BUDGETS: &[PathBudget] = &[
    // Lowered from 1,166 by `decompose-api-tool-use-loop`, which took six seams out of
    // `execute_with_code_intelligence` and dropped it from 978 lines to 621. What is left is the
    // tool-use loop itself plus the two helpers only it has — the SSE round and the tool-outcome
    // tail. Four further seams were available on a line count and refused: the non-success HTTP
    // handler (its recovery `continue` silently consumes a round trip), the per-tool-call dispatch
    // chain (seven branches ending in `continue`, two mutating the image counter), the remaining
    // setup bindings (no boundary, just position), and the two 27-line `maybe_compact_accounted`
    // calls (no reduction, only indirection). See that change's design.md.
    PathBudget {
        path:
            "src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/execution.rs",
        budget: 955,
        owner: "decompose-api-tool-use-loop",
    },
    // The other residual `split-api-adapter-modules` left above 1,000 lines: 43 native tool
    // implementations, the largest of which is `execute_tool_call_impl`'s 266-line dispatch.
    // Every other module the split produced is small enough for the subtree budget alone.
    PathBudget {
        path:
            "src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter/native_tools.rs",
        budget: 1_478,
        owner: "split-api-adapter-modules",
    },
    // Lowered from 5,110 by `relocate-heavyweight-inline-tests`, which split seven subject
    // modules out into `tests/`. What stays is the scaffolding they share — `Fixture`, the
    // record builders, and the evidence/logging port doubles — plus the tests interleaved with
    // it. The entry survives the split rather than being deleted: the file is still real, and
    // nothing else bounds its regrowth (this subtree has no registered subtree budget).
    // Raised by 1 for `provider_thread_id` on the one `SessionSeat` literal this file builds.
    PathBudget {
        path: "src-tauri/src/contexts/sessions/infrastructure/tests.rs",
        budget: 844,
        owner: "relocate-heavyweight-inline-tests",
    },
    // Lowered from 4,628 by the same change. ~1,600 of what remains is the single `FakeWorld`
    // port double and its ~25 impls. That is one cohesive test double, not a bucket, so it was
    // deliberately left whole — see the change's design.md.
    // Raised by 21 for `FakeWorld`'s `update_seat_provider_thread_id`, then by 31 more for its
    // `clear_seat_provider_thread_id` and `clear_runtime_session_id`. The double has to record
    // these per seat, not just accept them: a stub that dropped the write would let both the
    // seat-scoped capture and the discard pass while storing nothing, which is exactly the defect
    // they exist to catch.
    PathBudget {
        path: "src-tauri/src/contexts/agent_runtime/application/tests.rs",
        budget: 1_903,
        owner: "relocate-heavyweight-inline-tests",
    },
    PathBudget {
        path: "src-tauri/src/contexts/tooling/skills/application/tests.rs",
        budget: 4_049,
        owner: "relocate-heavyweight-inline-tests",
    },
];

const NATIVE_SUBTREE_BUDGETS: &[SubtreeBudget] = &[
    // Raised from 58,072 by `split-api-adapter-modules`, which turned `api_process_adapter.rs`
    // into a directory module. The +285 is entirely per-file scaffolding: +228 `use` lines (each
    // of the eight new modules carries its own explicit import list instead of inheriting the
    // parent's, plus `mod.rs`'s re-export blocks), +22 `#[cfg(test)]` attributes on the re-exports
    // that exist only for `tests.rs`'s `use super::*;`, +11 blank separators, +8 module docs,
    // +8 `mod` declarations, +4 comment lines heading the two re-export blocks, and +4 from
    // rustfmt rewrapping two signatures that a `pub(super)` qualifier pushed past 100 columns.
    // No item body was duplicated or edited: the top-level item multiset is identical before and
    // after, 84 of the 138 moved items are byte-identical to their pre-split text, 52 differ only
    // by an added `pub(super)`, and the last 2 are the rustfmt rewraps above.
    // Raised from 58,357 by two changes that landed together, and the two halves differ in kind.
    //
    // `freeze-panic-shortcuts-in-production-code` adds 5: the four-line
    // `#![allow(clippy::unwrap_used, clippy::expect_used)]` header plus its blank separator on
    // `runner_registry.rs`, the one file here carrying a pre-existing panic shortcut. That header
    // is the debt marker itself, so it falls again when the shortcut is retired.
    //
    // `decompose-api-tool-use-loop` adds 648, and unlike the split that set the previous figure it
    // is not a pure move:
    //
    // +382 is the six characterization tests and the `RejectingSink` they need, written against
    // the un-split function so each seam had coverage before it was cut - `CapturingSink` never
    // fails, `Effect::Deny` and `ApprovalOutcome::Answered(_)` had no test, and no test in the
    // suite had ever set `endpoint_profile: Some(..)`. The diff on `tests.rs` is additions only.
    //
    // +266 is production, and none of it is a duplicated body: 132 lines of doc comments,
    // `#[allow]` attributes and parameter lists across eleven new declarations; 17 lines of struct
    // and enum bodies for the three result types the seams return; 16 for `endpoint.rs`'s module
    // doc and import block; 7 net new `use` lines in the three existing modules that gained a
    // helper; and 94 of caller-side `match`/destructure scaffolding at eight call sites, already
    // net of the 40 lines saved by de-duplicating the five copies of the tool-outcome tail.
    //
    // Raised from 59,010 by the PTY-lifecycle fixes from the 2026-08-19 end-to-end pass. +223, of
    // which 163 are regression tests and 60 are production:
    //
    // +118 in `terminal_process.rs`'s test module: 54 for
    // `a_blocked_terminal_writer_does_not_hold_the_registry_lock`, 52 for
    // `a_read_that_decodes_to_no_text_still_reaches_the_provider_framer`, and 12 for splitting the
    // `managed_terminal` fixture into a `managed_terminal_named` that can build a second terminal
    // (the lock test needs two) plus its `TerminalIo` construction.
    //
    // +45 in `subagent_worktree_tests.rs` for
    // `a_reap_that_falls_back_to_the_filesystem_leaves_no_administrative_record`, which covers
    // git's administrative record on the reap path where the directory is already gone —
    // `the_worktree_is_reaped_when_dropped` only ever covered the files.
    //
    // +60 production, none of it a duplicated body: 22 for `split_terminal_read` and its doc,
    // which is the extracted per-read step whose old inline form dropped a read's bytes from the
    // provider framer whenever that read decoded to no displayable text; 15 for
    // `checkout_terminal_io` and its doc; 13 for the `TerminalIo` struct and its doc; 8 for the
    // `checkout_io` method; and 2 net across the construction site, the reader loop and the
    // `input`/`resize` rewrites that stopped holding the registry lock across blocking PTY calls.
    //
    // Raised again from 59,233 by +46, all of it one regression test:
    // `claude_code_unrecognised_structured_events_are_not_emitted_as_text` in
    // `providers/tests.rs`. It pins the case where a claude-code turn's eight `stream_event`
    // wrappers and its `rate_limit_event` were emitted as the Agent's own words, because the
    // parser's fallback treated any unrecognised line as literal output. The production side of
    // that fix replaced one match arm and added no net lines.
    //
    // Raised again from 59,279 by +24, all of it explanation rather than logic. gemini-cli moved
    // from argv to stdin prompt delivery (+6 net in `providers/invocation.rs`, a shorter arm under
    // a longer comment), the spawn path now carries the OS failure through `RunnerError::detail`
    // (+3 in `local_runner.rs`), and `process_adapter.rs` renders that detail into the lifecycle
    // log (+12, a small helper and its doc). The reasons are worth the lines: each one is a defect
    // that cost a full investigation to find because the code said what it did and not why.
    //
    // Raised again from 59,303 by +4: adding the `detail` parameter above took
    // `record_runner_lifecycle` past clippy's argument threshold, and the suppression carries a
    // reason rather than standing bare, since the wrapper one function above suppresses the same
    // lint for the same list and a reader deserves to know why both are acceptable.
    //
    // Raised again from 59,307 by +11 in `terminal_wrapper.rs`: its token validator rejected only
    // NUL, while every token it admits is written into a script file that an interpreter reads
    // back, and a batch file has no escape for a raw newline. Six lines record why this validator
    // is stricter than the argv one, and five extend the test to the rest of the control range.
    //
    // Raised again from 59,318 to 59,425 by +107, which is two changes rather than one.
    //
    // +23 is `fix(agent-runtime): resolve an Agent's executable the way its launch does`
    // (4e7a0d4b), which should have carried its own raise and did not: availability checking moved
    // off a bare `command_exists` onto the same `CliApi::resolve_executable` the launch uses, so
    // the gateway now holds a `CliApi` and its port takes the `agent_id` that resolution needs.
    // An Agent installed anywhere but the default PATH entry reported unavailable while launching
    // fine.
    //
    // +84 is `BuiltinAwareExpertRoleRepository`: 51 production, of which 12 are the decorator's
    // struct, constructor and three trait methods and 39 are the doc comments recording why the
    // merge lives at the port rather than in the roster; and 33 for the regression test. The seat
    // roster resolved a seat's role through the bare SQLite port, which holds only stored roles,
    // so the three roles the product actually ships resolved to nothing, seats fell back to being
    // named after their Agent, and `@架构师` addressed nobody -- multi-Agent handoff silently
    // stopped relaying for the default configuration.
    //
    // Raised again from 59,425 by +12 for `scope-provider-resume-metadata-to-a-seat`: 11 for the
    // sessions gateway's `update_seat_provider_thread_id` and its mapping of the seat's own thread
    // id, and 1 in the API adapter's test request. The generation adapter is net zero -- it stops
    // reading `session.runtime_session_id` and reads the already-resolved id instead.
    //
    // Raised again by +19 for the same change's recovery half: the gateway's two clearing methods,
    // which let a turn that failed to resume forget the thread so the seat is not stuck resuming a
    // dead id on every turn thereafter.
    // Raised from 59,456 by +11 for `prevent-hook-bash-permission-blocks`: 3 lines inject the
    // VaneHub-owned Claude Code scope marker, and 8 lines verify chat and terminal projections add
    // it only for Claude Code across the complete managed-CLI policy matrix.
    //
    // Raised from 59,467 to 60,547 by `upgrade-cli-parameter-management`'s native runtime cutover.
    // The subtree grows 1,476 lines against `ee3eaf3f`; 396 fit the existing headroom, so the
    // budget rises by 1,080. Production here does not grow at all — it falls by 328 — and every
    // line of the raise is `#[cfg(test)]`:
    //
    // +693 `baseline_argv_equivalence_tests.rs`, which transcribes the pre-cutover
    // `build_invocation`, `build_interactive_invocation`, `apply_policy_template_overrides` and
    // `force_gemini_standard_approval_flag` verbatim from `ee3eaf3f`, recomputes each provider's
    // argv through the legacy renderer, and asserts equality against the live resolver for all
    // five providers across interactive, fresh chat and resume. Its duplication of the old bodies
    // is the point: without a second, independent computation of the old argv there is no way to
    // show the cutover preserved it rather than to assert that it did. It also pins the only two
    // differences that are intended, so a third one cannot appear silently. It falls with the
    // legacy monolith.
    //
    // +932 `cli_profile_tests.rs`, which is `cli_profile.rs`'s own `mod tests` moved out (that
    // move is most of the -327 on `cli_profile.rs`) and extended to 23 tests: the policy
    // projection per agent and template, the legacy and v2 read paths, quarantine that does not
    // fail a launch, launch-time re-evaluation of profile, policy and CLI version, and the
    // diagnostics' operation association and freedom from prompts, credentials and session ids.
    //
    // +179 across `providers/tests.rs`, `compatibility_tests.rs` and the three JSON fixtures for
    // the table-driven runtime coverage the change requires.
    //
    // -328 production: `invocation.rs` loses its per-parameter-id renderer branches and
    // `cli_profile.rs` its duplicate `default` interpretation, both now owned by the tooling
    // resolver.
    //
    // Raised from 60,547 by `harden-provider-output-oversized-records`: the provider output
    // framer gains skip-and-resume handling for oversized records plus a discard counter
    // (`providers/output.rs`, production and reworked framer tests), and the generation read
    // loop reports discarded records to unified logs after the stream ends
    // (`process_adapter.rs`). No code was moved or duplicated; the raise is the resilience
    // logic and its tests.
    SubtreeBudget {
        root: "src-tauri/src/contexts/agent_runtime/infrastructure",
        budget: 60_665,
        owner: "harden-provider-output-oversized-records",
    },
    // Raised from 2,914 by `split-database-migrations`, which turned `migrations.rs` into a
    // directory module. The +51 is entirely per-file boilerplate: +29 module headers (the `mod`
    // declarations, the `use inline_schema::{…}` re-import list, and `inline_schema.rs`'s module
    // doc and imports), +28 for rustfmt wrapping 14 `pub(super) fn` signatures that now exceed
    // 100 columns, less 5 for the `mod tests { … }` wrapper disappearing and 1 blank separator.
    // No migration body was duplicated — every one of them moved byte-identically.
    SubtreeBudget {
        root: "src-tauri/src/platform/database",
        budget: 2_965,
        owner: "split-database-migrations",
    },
];

/// Production-only ceilings. Set from the measurement taken when the entry was added, so they can
/// only be raised by an explicit decision about production code — never as a side effect of adding
/// tests.
const NATIVE_PRODUCTION_SUBTREE_BUDGETS: &[SubtreeBudget] = &[
    // `upgrade-cli-parameter-management` raised the aggregate ceiling for this subtree by 1,080
    // lines of characterization tests. That raise must not become production headroom, so this
    // records what production actually measured on the same commit.
    SubtreeBudget {
        root: "src-tauri/src/contexts/agent_runtime/infrastructure",
        // The first version of this entry read 26,998, because the measurement truncated each file
        // at its first `#[cfg(test)]` and several files declare `#[cfg(test)] mod tests;` near the
        // top and continue with production code below. That discarded 5,966 real production lines
        // and left the subtree that much silent headroom — the opposite of what a ceiling is for.
        // Raised from 32,964 by `harden-provider-output-oversized-records`: the framer's
        // skip-and-resume path and discard counter in `providers/output.rs`, and the
        // post-stream discarded-records warn log in `process_adapter.rs`, are production code
        // measured on the same commit.
        budget: 33_049,
        owner: "harden-provider-output-oversized-records",
    },
];

fn physical_lines(source: &str) -> usize {
    source.lines().count()
}

fn measure_budgeted_path(root: &Path, relative: &str) -> Option<usize> {
    let path = root.join(relative);
    if !path.is_file() {
        return None;
    }
    Some(physical_lines(
        &fs::read_to_string(&path).expect("read budgeted source"),
    ))
}

fn measure_budgeted_subtree(root: &Path, relative: &str) -> usize {
    rust_files(&root.join(relative))
        .expect("enumerate budgeted subtree")
        .iter()
        .map(|path| physical_lines(&fs::read_to_string(path).expect("read budgeted source")))
        .sum()
}

/// Production lines only.
///
/// The aggregate budget above counts tests too, so raising it for a characterization suite silently
/// hands the same number of lines to production. This measurement is what stops that: a subtree can
/// grow a thousand lines of tests without gaining room for one line of production code.
///
/// Truncating at the first `#[cfg(test)]` would have been wrong, and provably so — several files
/// declare `#[cfg(test)] mod tests;` near the top and continue with production code below it, which
/// a truncating count would have discarded. Test regions are therefore matched by brace instead,
/// and anything the matcher is not certain about is counted as production: a ceiling that is too
/// tight forces an explicit decision, whereas one that is too loose grants silent headroom.
fn production_lines(source: &str) -> usize {
    let lines: Vec<&str> = source.lines().collect();
    let mut counted = 0usize;
    let mut index = 0usize;
    while index < lines.len() {
        if lines[index].trim() != "#[cfg(test)]" {
            counted += 1;
            index += 1;
            continue;
        }
        let mut next = index + 1;
        while next < lines.len() && lines[next].trim().is_empty() {
            next += 1;
        }
        let Some(declaration) = lines.get(next) else {
            counted += 1;
            index += 1;
            continue;
        };
        let trimmed = declaration.trim();
        if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
            // A test module declared in its own file. The file itself is skipped by name; only the
            // two lines that point at it are test-only here.
            index = next + 1;
            continue;
        }
        if trimmed.starts_with("mod ") && trimmed.ends_with('{') {
            let indent = declaration.len() - declaration.trim_start().len();
            let closing = format!("{}}}", " ".repeat(indent));
            let mut scan = next + 1;
            while scan < lines.len() && lines[scan] != closing {
                scan += 1;
            }
            index = if scan < lines.len() {
                scan + 1
            } else {
                lines.len()
            };
            continue;
        }
        // A `#[cfg(test)]` on something other than a module: counted, deliberately.
        counted += 1;
        index += 1;
    }
    counted
}

fn measure_production_subtree(root: &Path, relative: &str) -> usize {
    let subtree = root.join(relative);
    rust_files(&subtree)
        .expect("enumerate production subtree")
        .iter()
        .filter(|path| {
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            !is_test_source(&relative_path)
        })
        .map(|path| production_lines(&fs::read_to_string(path).expect("read production source")))
        .sum()
}

#[test]
fn the_production_measurement_skips_test_modules_without_swallowing_the_code_after_them() {
    let source = "\
fn kept_before() {}

#[cfg(test)]
mod inline {
    fn hidden() {}
}

#[cfg(test)]
mod declared;

fn kept_after() {}
";

    // Seven counted lines: the two `fn` declarations, and the blank separators that survive around
    // them. What matters is that `kept_after` is not discarded.
    assert!(production_lines(source) < physical_lines(source));
    assert!(source.contains("fn kept_after"));
    assert_eq!(production_lines("fn only() {}\n"), 1);
    assert_eq!(production_lines("#[cfg(test)]\nmod declared;\n"), 0);
}

/// `None` for `measured` means the path is gone. That is satisfied, not skipped: the subtree
/// budget bounds whatever replaced it, so a rename cannot dissolve the ceiling.
fn path_budget_diagnostic(budget: &PathBudget, measured: Option<usize>) -> Option<String> {
    let measured = measured?;
    (measured > budget.budget).then(|| {
        format!(
            "[ARCH-NATIVE-006] {}: {measured} physical lines exceeds budget {}. Owner: {}. \
             Repair: reduce the change, or raise the budget in the same commit and state why",
            budget.path, budget.budget, budget.owner
        )
    })
}

fn subtree_budget_diagnostic(budget: &SubtreeBudget, measured: usize) -> Option<String> {
    (measured > budget.budget).then(|| {
        format!(
            "[ARCH-NATIVE-007] {}: {measured} aggregate physical lines exceeds budget {}. \
             Owner: {}. Repair: a split must move code, not duplicate it; raise the budget in \
             the same commit only with a stated reason",
            budget.root, budget.budget, budget.owner
        )
    })
}

#[test]
fn oversized_native_paths_stay_within_their_recorded_line_budgets() {
    let root = project_root();
    let mut violations = Vec::new();
    let mut present_paths = 0usize;

    for budget in NATIVE_PATH_BUDGETS {
        let measured = measure_budgeted_path(&root, budget.path);
        if measured.is_some() {
            present_paths += 1;
        }
        violations.extend(path_budget_diagnostic(budget, measured));
    }

    for budget in NATIVE_SUBTREE_BUDGETS {
        let measured = measure_budgeted_subtree(&root, budget.root);
        violations.extend(subtree_budget_diagnostic(budget, measured));
    }

    for budget in NATIVE_PRODUCTION_SUBTREE_BUDGETS {
        let measured = measure_production_subtree(&root, budget.root);
        if measured > budget.budget {
            violations.push(format!(
                "[ARCH-NATIVE-008] {}: {measured} production physical lines exceeds budget {}. \
                 Owner: {}. Repair: this ceiling skips test files and brace-matched test \
                 modules and nothing else, so a test-driven raise of the aggregate budget does \
                 not move it",
                budget.root, budget.budget, budget.owner
            ));
        }
    }

    assert!(
        present_paths > 0,
        "every budgeted path is missing, so this guard is asserting nothing"
    );
    assert!(
        violations.is_empty(),
        "recorded line budgets exceeded:\n{}",
        violations.join("\n")
    );
}

#[test]
fn line_budget_detector_accepts_paths_and_subtrees_within_budget() {
    let path = PathBudget {
        path: "within.rs",
        budget: 10,
        owner: "some-change",
    };
    let subtree = SubtreeBudget {
        root: "src/within",
        budget: 10,
        owner: "some-change",
    };

    assert_eq!(path_budget_diagnostic(&path, Some(10)), None);
    assert_eq!(subtree_budget_diagnostic(&subtree, 9), None);
}

#[test]
fn line_budget_detector_reports_a_path_over_budget_with_measurement_and_budget() {
    let budget = PathBudget {
        path: "oversized.rs",
        budget: 100,
        owner: "split-oversized",
    };

    let diagnostic = path_budget_diagnostic(&budget, Some(101)).expect("path over budget");

    assert!(diagnostic.starts_with("[ARCH-NATIVE-006] oversized.rs:"));
    assert!(diagnostic.contains("101 physical lines"));
    assert!(diagnostic.contains("budget 100"));
    assert!(diagnostic.contains("Owner: split-oversized"));
}

#[test]
fn line_budget_detector_names_the_subtree_rather_than_an_individual_file() {
    let budget = SubtreeBudget {
        root: "src/oversized",
        budget: 50,
        owner: "split-oversized",
    };

    let diagnostic = subtree_budget_diagnostic(&budget, 51).expect("subtree over budget");

    assert!(diagnostic.starts_with("[ARCH-NATIVE-007] src/oversized:"));
    assert!(diagnostic.contains("51 aggregate physical lines"));
    assert!(!diagnostic.contains(".rs"));
}

#[test]
fn line_budget_detector_treats_a_missing_path_as_satisfied_while_its_subtree_still_binds() {
    let path = PathBudget {
        path: "split-away.rs",
        budget: 1,
        owner: "split-away",
    };
    let subtree = SubtreeBudget {
        root: "src/split-away",
        budget: 1,
        owner: "split-away",
    };

    assert_eq!(path_budget_diagnostic(&path, None), None);
    assert!(subtree_budget_diagnostic(&subtree, 2).is_some());
}

#[test]
fn physical_line_counter_matches_newline_terminated_counting() {
    assert_eq!(physical_lines("a\nb\nc\n"), 3);
    assert_eq!(physical_lines("a\nb\nc"), 3);
    assert_eq!(physical_lines(""), 0);
}

/// `registry.rs` routes an invoke by *name*: `supplemental_registry::is_command` decides whether a
/// command reaches the supplemental handler at all, and anything it does not name falls through to
/// the core handler, which does not have it. A command registered in `generate_handler!` but
/// missing from that name list is therefore dead at runtime -- `Command <name> not found` -- and
/// nothing in the type system notices, because the two lists never reference each other.
///
/// Found this way: the Goals screen rendered its error banner on a real desktop run while every
/// goals command sat correctly in the handler macro.
#[test]
fn supplemental_registry_routes_every_command_it_registers() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/supplemental_registry.rs"),
    )
    .expect("read supplemental registry");

    let registered = registered_supplemental_commands(&source);
    assert!(
        registered.len() > 20,
        "the registry parser found only {} commands, so it has stopped matching the source",
        registered.len()
    );
    let routed = routed_supplemental_commands(&source);

    let unroutable = registered
        .iter()
        .filter(|command| !routed.contains(*command))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        unroutable.is_empty(),
        "[ARCH-NATIVE-008] registered but never routed, so every call answers \
         `Command <name> not found`:\n{}",
        unroutable.join("\n")
    );

    let unregistered = routed
        .iter()
        .filter(|command| !registered.contains(*command))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        unregistered.is_empty(),
        "[ARCH-NATIVE-008] routed to the supplemental handler but not registered in it, so the \
         call reaches a handler that cannot answer it:\n{}",
        unregistered.join("\n")
    );
}

/// Final path segment of every entry inside `generate_handler![ ... ]`.
fn registered_supplemental_commands(source: &str) -> Vec<String> {
    let start = source
        .find("generate_handler![")
        .map(|index| index + "generate_handler![".len())
        .expect("supplemental registry declares a handler");
    let mut depth = 1usize;
    let mut end = start;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    source[start..end]
        .lines()
        .filter_map(|line| {
            let entry = line.trim().trim_end_matches(',');
            (!entry.is_empty() && !entry.starts_with("//"))
                .then(|| entry.rsplit("::").next().unwrap_or(entry).to_string())
        })
        .collect()
}

/// The command names in the `is_command` name list.
///
/// Every quoted span after `fn is_command` is not the same thing as every name it routes: the
/// registry's own `#[cfg(test)]` block sits after that function, and its assertion messages are
/// string literals too. Harvesting those reported prose like `unknown command` as a routed
/// command, which failed this test against a registry that was in fact consistent.
///
/// So the scan stops at the test module, and keeps only lines whose whole content is one quoted
/// command name -- optionally behind the `|` of a `matches!` arm -- which is the shape the list is
/// actually written in. That is the same discipline
/// `supplemental_registry::tests::every_registered_supplemental_command_is_also_routed_to` applies
/// to the same list; the two parsers disagreeing is what let this drift through.
fn routed_supplemental_commands(source: &str) -> Vec<String> {
    let start = source
        .find("fn is_command")
        .expect("supplemental registry declares a name-based router");
    let body = &source[start..];
    let body = match body.find("#[cfg(test)]") {
        Some(end) => &body[..end],
        None => body,
    };
    body.lines()
        .map(|line| line.trim().trim_start_matches('|').trim())
        .filter_map(|line| line.strip_prefix('"')?.strip_suffix('"'))
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        })
        .map(str::to_string)
        .collect()
}
