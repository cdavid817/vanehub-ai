const COMMANDS: [(&str, &str); 7] = [
    (
        "list_evaluation_tasks",
        include_str!("list_evaluation_tasks.rs"),
    ),
    ("start_evaluation", include_str!("start_evaluation.rs")),
    (
        "list_evaluation_arenas",
        include_str!("list_evaluation_arenas.rs"),
    ),
    (
        "get_evaluation_arena",
        include_str!("get_evaluation_arena.rs"),
    ),
    ("cancel_evaluation", include_str!("cancel_evaluation.rs")),
    (
        "get_evaluation_attempt",
        include_str!("get_evaluation_attempt.rs"),
    ),
    ("export_evaluation", include_str!("export_evaluation.rs")),
];

#[test]
fn every_evaluation_operation_has_one_thin_tauri_command() {
    for (name, source) in COMMANDS {
        assert!(source.contains("#[tauri::command]"), "{name}");
        assert!(source.contains("State<'_, EvaluationApi>"), "{name}");
        assert!(!source.contains("rusqlite"), "{name}");
        assert!(!source.contains("invoke("), "{name}");
    }
}

#[test]
fn command_registry_contains_every_evaluation_operation() {
    let registry = include_str!("../core_registry.rs");
    for (name, _) in COMMANDS {
        assert!(registry.contains(&format!("::{name}")), "{name}");
    }
}
