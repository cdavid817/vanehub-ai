use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Expr, ExprLit, ImplItem, Item, ItemFn, ItemUse, Lit, UseTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Domain,
    Application,
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
    rule: &'static str,
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
        if is_forbidden_technology(segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule: "domain/application code cannot depend on concrete I/O or runtime frameworks",
            });
            return;
        }
        if is_forbidden_outer_layer(self.scope, segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule: "dependencies must point inward from adapters to application and domain",
            });
            return;
        }
        if imports_private_cross_context_module(self.scope, segments) {
            self.violations.insert(Violation {
                line,
                dependency,
                rule: "cross-context access must use the owning context api module",
            });
        }
    }
}

impl<'ast> Visit<'ast> for DependencyVisitor<'_> {
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
}

fn source_scope(relative_path: &Path) -> Option<SourceScope> {
    let parts = relative_path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let contexts = parts.iter().position(|part| part == "contexts")?;
    let context = parts.get(contexts + 1)?.clone();
    let layer = match parts.get(contexts + 2).map(String::as_str) {
        Some("domain") => Layer::Domain,
        Some("application") => Layer::Application,
        _ => return None,
    };
    Some(SourceScope { context, layer })
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
                "{}:{}: {} (`{}`)",
                relative.display(),
                violation.line,
                violation.rule,
                violation.dependency
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
        violation.line == 2 && violation.dependency.starts_with("rusqlite::Connection")
    }));
    assert!(violations.iter().any(|violation| {
        violation.line == 3
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
                "{relative}: io {}/{}; control flow {}/{}",
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
                    relative == "platform/logging.rs"
                }
                _ => false,
            };
            if !allowed {
                violations.push(format!("{relative}:{}: {}", usage.line, usage.kind));
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
