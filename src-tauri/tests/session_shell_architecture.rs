//! The Session Shell lifecycle has exactly one owner.
//!
//! These guards exist because the one-view shell service was retired, not merely stopped being
//! called. A retired subsystem that is still constructible comes back: the next person who needs a
//! shell finds two ways to make one, picks the one whose name they recognise, and the capability
//! this change built — a shell that survives a tab switch — quietly stops applying to whatever they
//! wrote. Every check below names a specific way that could happen.

use std::fs;
use std::path::{Path, PathBuf};

/// The one-view command names. Registering any of them again would put a second, non-retained shell
/// lifecycle back on the IPC surface.
const RETIRED_SHELL_COMMANDS: &[&str] = &[
    "shell_create",
    "shell_input",
    "shell_cd",
    "shell_resize",
    "shell_kill",
];

/// The retired types. Named rather than pattern-matched: a check that looked for "anything shaped
/// like a shell service" would also flag the retained registry.
const RETIRED_SHELL_TYPES: &[&str] = &[
    "WorkspaceShellApplicationService",
    "PortablePtyShellRuntime",
    "WorkspaceShellRuntimePort",
    "WorkspaceShellEventPort",
    "WorkspaceShellIdPort",
];

/// The frontend methods the one-view service exposed.
const RETIRED_FRONTEND_METHODS: &[&str] = &[
    "createShell",
    "writeShellInput",
    "resetShellDirectory",
    "resizeShell",
    "killShell",
    "subscribeShellEvents",
];

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn files(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "node_modules") {
                    continue;
                }
                pending.push(path);
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| extensions.contains(&value))
            {
                found.push(path);
            }
        }
    }
    found
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Test sources may still name a retired symbol to assert it is gone; production may not.
fn is_test_path(relative: &str) -> bool {
    relative.contains("/tests/")
        || relative.starts_with("tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with(".test.ts")
        || relative.ends_with(".test.tsx")
        || relative.contains("/test/")
}

#[test]
fn production_registers_no_legacy_one_view_shell_commands() {
    let root = project_root();
    let registry = root.join("src-tauri/src/commands");
    let mut violations = Vec::new();
    let mut inspected = 0;
    for path in files(&registry, &["rs"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) {
            continue;
        }
        inspected += 1;
        let source = fs::read_to_string(&path).expect("read command source");
        for command in RETIRED_SHELL_COMMANDS {
            // `shell_kill` is also an Agent-facing native tool name, which is a different thing
            // entirely and stays. Only a Tauri command registration is a violation.
            if source.contains(&format!("::{command}::{command}"))
                || source.contains(&format!("#[tauri::command]\npub(crate) fn {command}("))
            {
                violations.push(format!(
                    "{name}: registers retired shell command `{command}`"
                ));
            }
        }
    }
    assert!(inspected > 0, "no command sources were inspected");
    assert!(
        violations.is_empty(),
        "the retired one-view shell commands came back:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_constructs_no_one_view_shell_service() {
    let root = project_root();
    let mut violations = Vec::new();
    let mut inspected = 0;
    for path in files(&root.join("src-tauri/src"), &["rs"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) {
            continue;
        }
        inspected += 1;
        let source = fs::read_to_string(&path).expect("read native source");
        for retired in RETIRED_SHELL_TYPES {
            if source.contains(retired) {
                violations.push(format!("{name}: names retired shell type `{retired}`"));
            }
        }
    }
    assert!(inspected > 0, "no native sources were inspected");
    assert!(
        violations.is_empty(),
        "a retired one-view shell type is still reachable:\n{}",
        violations.join("\n")
    );
}

/// One lifecycle owner, checked by construction rather than by convention: `SessionShellRegistry`
/// is the only thing that opens or closes a Session Shell, so there is no second place for a Shell
/// to be created that the registry does not know about — which is what would make it un-retained.
#[test]
fn the_retained_registry_is_the_only_session_shell_lifecycle_owner() {
    let root = project_root();
    let registry =
        root.join("src-tauri/src/contexts/workspaces/application/session_shell_registry.rs");
    let source = fs::read_to_string(&registry).expect("read the retained registry");
    assert!(
        source.contains("fn create(") && source.contains("fn close_with("),
        "the retained registry no longer owns create and close"
    );

    let mut owners = Vec::new();
    for path in files(&root.join("src-tauri/src"), &["rs"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) || name.ends_with("session_shell_registry.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read native source");
        // Calling the runtime port's `open` outside the registry would be a second creation path.
        if source.contains(".runtime.open(") {
            owners.push(name);
        }
    }
    assert!(
        owners.is_empty(),
        "a Session Shell is opened outside the retained registry:\n{}",
        owners.join("\n")
    );
}

/// Shell evidence has one producer. Two would double-count a session's shells, and a reader has no
/// way to tell a double count from a session that really did open twice as many.
#[test]
fn the_retained_registry_is_the_only_shell_evidence_producer() {
    let root = project_root();
    let mut producers = Vec::new();
    for path in files(&root.join("src-tauri/src"), &["rs"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read native source");
        if source.contains("WorkspaceEvidenceSignal::ShellOpened")
            || source.contains("WorkspaceEvidenceSignal::ShellClosed")
        {
            producers.push(name);
        }
    }
    producers.sort();
    // The enum's own definition declares the variants; the registry is the only thing that builds
    // them, and the bootstrap adapter that translates them reads rather than constructs.
    let expected =
        ["src-tauri/src/contexts/workspaces/application/session_shell_registry.rs".to_string()];
    let built: Vec<String> = producers
        .into_iter()
        .filter(|name| !name.ends_with("application/evidence.rs"))
        .filter(|name| !name.starts_with("src-tauri/src/bootstrap/"))
        .collect();
    assert_eq!(
        built, expected,
        "Shell evidence is produced somewhere other than the retained registry"
    );
}

/// A call or a declaration, not a mention.
///
/// `createShellFrameDispatcher` belongs to the retained client and shares a prefix with a retired
/// name; matching on the trailing `(` is what tells them apart, so the guard does not have to
/// exclude the file that legitimately contains it.
fn retired_frontend_methods(source: &str) -> Vec<&'static str> {
    RETIRED_FRONTEND_METHODS
        .iter()
        .copied()
        .filter(|method| {
            source.contains(&format!("{method}(")) || source.contains(&format!("\"{method}\""))
        })
        .collect()
}

#[test]
fn the_frontend_exposes_no_legacy_one_view_shell_methods() {
    let root = project_root();
    let mut violations = Vec::new();
    let mut inspected = 0;
    for path in files(&root.join("src"), &["ts", "tsx"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) {
            continue;
        }
        inspected += 1;
        let source = fs::read_to_string(&path).expect("read frontend source");
        for method in retired_frontend_methods(&source) {
            violations.push(format!("{name}: still exposes `{method}`"));
        }
    }
    assert!(inspected > 0, "no frontend sources were inspected");
    assert!(
        violations.is_empty(),
        "the retired one-view shell methods are still on the service boundary:\n{}",
        violations.join("\n")
    );
}

/// The guard above is only worth having if it would fire. A detector that quietly matches nothing
/// reads exactly like a codebase that is clean.
#[test]
fn the_frontend_detector_separates_a_retired_method_from_the_retained_dispatcher() {
    assert_eq!(
        retired_frontend_methods("await agentService.killShell(shellId);"),
        vec!["killShell"]
    );
    assert_eq!(
        retired_frontend_methods("  createShell(input: CreateShellInput): Promise<ShellSession>;"),
        vec!["createShell"]
    );
    assert!(
        retired_frontend_methods("const d = createShellFrameDispatcher({ shellId });").is_empty(),
        "the retained frame dispatcher must not read as a retired method"
    );
    assert!(
        retired_frontend_methods("await sessionShellService.resizeSessionShell(input);").is_empty(),
        "the retained resize must not read as the retired one"
    );
}

/// A Session Shell is the user's own terminal; an Agent Terminal is a runtime the Agent drives.
/// They share a PTY crate and nothing else, and merging them would let one capability's lifecycle
/// decide the other's — a tab switch ending an Agent run, or an Agent's stop killing the user's
/// build.
#[test]
fn session_shell_and_agent_terminal_stay_separate() {
    let root = project_root();
    let shell_files = [
        "src-tauri/src/contexts/workspaces/application/session_shell_registry.rs",
        "src-tauri/src/contexts/workspaces/application/session_shell_store.rs",
        "src-tauri/src/contexts/workspaces/infrastructure/retained_shell_runtime.rs",
        "src-tauri/src/contexts/workspaces/infrastructure/retained_remote_shell.rs",
    ];
    let mut violations = Vec::new();
    for relative_path in shell_files {
        let source = fs::read_to_string(root.join(relative_path))
            .unwrap_or_else(|_| panic!("read {relative_path}"));
        if source.contains("agent_runtime") {
            violations.push(format!("{relative_path}: reaches into agent_runtime"));
        }
    }

    let terminal_root = root.join("src-tauri/src/contexts/agent_runtime");
    for path in files(&terminal_root, &["rs"]) {
        let name = relative(&root, &path);
        if is_test_path(&name) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read agent runtime source");
        if source.contains("SessionShellRegistry") || source.contains("session_shell") {
            violations.push(format!("{name}: reaches into the Session Shell registry"));
        }
    }
    assert!(
        violations.is_empty(),
        "Session Shell and Agent Terminal are no longer separate:\n{}",
        violations.join("\n")
    );
}

/// The runtime descriptor stays narrowable by `kind`.
///
/// The string capability union this replaced let the UI offer resize to a simulated shell and
/// reconnect to a PTY, because a string carries no constraints. The union that replaced it carries
/// each capability inside the variant that has it — but the field the frontend narrows on exists
/// only because of the serde attribute. Remove the attribute and serde falls back to external
/// tagging, `{"native": {…}}`, at which point `kind` is `undefined` on every descriptor and every
/// narrow falls through to its last branch.
///
/// Nothing else catches that. TypeScript will not: the frontend type is declared rather than
/// derived, so it goes on claiming a field the wire no longer carries, and the failure surfaces as
/// a shell that quietly reports the wrong capabilities rather than as an error anywhere.
#[test]
fn the_shell_runtime_descriptor_stays_narrowable_by_kind() {
    let dto = fs::read_to_string(project_root().join("src-tauri/src/commands/workspaces/dto.rs"))
        .expect("the workspaces DTO");

    let before_declaration = dto
        .split("pub(crate) enum ShellRuntimeDescriptor")
        .next()
        .expect("the descriptor is declared in the workspaces DTO");
    // The attributes immediately above the declaration, not the whole file: a `tag = "kind"` on
    // some other enum would otherwise satisfy this.
    let attributes = before_declaration
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        dto.contains("pub(crate) enum ShellRuntimeDescriptor"),
        "the descriptor was renamed; this rule is now checking nothing"
    );
    assert!(
        attributes.contains(r#"tag = "kind""#),
        "[ARCH-SHELL-006] the runtime descriptor is no longer internally tagged, so the \
         frontend's `runtime.kind` narrow reads undefined on every variant. Found: {attributes}"
    );

    // And the frontend still narrows on it, rather than having gone back to a bare string.
    let frontend = fs::read_to_string(project_root().join("src/types/session-workspace.ts"))
        .expect("the frontend workspace types");
    let declaration = frontend
        .split("export type ShellRuntimeDescriptor")
        .nth(1)
        .expect("the frontend declares the descriptor")
        .split("\n\n")
        .next()
        .unwrap_or_default();
    assert_eq!(
        declaration.matches("kind: \"").count(),
        4,
        "every variant carries its own `kind`, and there are four: {declaration}"
    );
}
