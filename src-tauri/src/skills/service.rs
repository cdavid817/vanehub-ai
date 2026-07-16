use crate::skills::models::*;
use crate::AppError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const VANEHUB_DIR: &str = ".vanehub";
const SKILLS_DIR: &str = "skills";
const SKILL_FILE: &str = "SKILL.md";

#[derive(Debug, Clone)]
struct BuiltinSkill {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: &'static str,
    triggers: &'static [&'static str],
    body: &'static str,
}

const BUILTIN_SKILLS: [BuiltinSkill; 6] = [
    BuiltinSkill {
        id: "tdd-discipline",
        name: "TDD 开发纪律助手",
        description: "引导开发过程遵循测试先行、红绿重构和回归验证纪律。",
        category: "development",
        triggers: &["TDD", "测试先行", "红绿重构"],
        body: "Use this skill to keep implementation work aligned with test-first development discipline. Start by identifying the behavior under change, add or update focused tests, implement the minimal code required, then run the relevant verification before broadening scope.",
    },
    BuiltinSkill {
        id: "code-review",
        name: "代码审查助手",
        description: "从缺陷、回归风险、可维护性和测试缺口角度审查代码变更。",
        category: "review",
        triggers: &["代码审查", "review", "检查变更"],
        body: "Use this skill for code review. Prioritize correctness, regressions, missing tests, data loss, security issues, and maintainability. Lead with concrete findings tied to files and behavior.",
    },
    BuiltinSkill {
        id: "code-security-scan",
        name: "代码安全扫描",
        description: "检查常见安全风险、敏感信息泄漏、命令注入和不安全文件操作。",
        category: "security",
        triggers: &["安全扫描", "security", "漏洞"],
        body: "Use this skill to review code for security risks. Check trust boundaries, shell/file operations, secrets handling, dependency usage, input validation, and authorization-sensitive paths.",
    },
    BuiltinSkill {
        id: "api-doc-generation",
        name: "API 文档自动生成",
        description: "根据接口、类型和示例生成结构化 API 文档。",
        category: "documentation",
        triggers: &["API 文档", "接口文档", "api docs"],
        body: "Use this skill to generate API documentation from source interfaces and examples. Include purpose, parameters, response shapes, errors, and practical usage examples.",
    },
    BuiltinSkill {
        id: "unit-test-generation",
        name: "单元测试自动生成",
        description: "为核心函数、边界条件和回归场景生成单元测试。",
        category: "testing",
        triggers: &["单元测试", "unit test", "测试生成"],
        body: "Use this skill to add focused unit tests. Cover expected behavior, edge cases, invalid input, and regressions. Keep tests close to existing project patterns.",
    },
    BuiltinSkill {
        id: "readme-generation",
        name: "README 文档生成",
        description: "生成或改进项目 README，包括安装、使用、开发和验证说明。",
        category: "documentation",
        triggers: &["README", "项目说明", "使用文档"],
        body: "Use this skill to create or improve README content. Cover what the project does, setup, common commands, configuration, development workflow, and troubleshooting.",
    },
];

#[derive(Debug, Clone)]
struct SkillRecord {
    id: String,
    scope: SkillScope,
    workspace_path: Option<String>,
    source: SkillSource,
    enabled: bool,
    skill_dir: String,
    skill_md_path: String,
    content_hash: String,
    metadata: SkillMetadata,
    created_at: String,
    updated_at: String,
}

pub fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skills (
            id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            skill_dir TEXT NOT NULL,
            skill_md_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            metadata_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (id, scope, workspace_path)
        );

        CREATE TABLE IF NOT EXISTS skill_agent_bindings (
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL,
            mounted_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, scope, workspace_path, agent_id)
        );

        CREATE TABLE IF NOT EXISTS skill_agent_mount_paths (
            agent_id TEXT PRIMARY KEY,
            mount_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS deleted_builtin_skills (
            skill_id TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS skill_drift_snapshots (
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            drift_hash TEXT NOT NULL,
            issues_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (scope, workspace_path)
        );
        "#,
    )?;
    Ok(())
}

pub fn seed_builtin_skills(conn: &Connection) -> Result<(), AppError> {
    for builtin in BUILTIN_SKILLS {
        if is_builtin_deleted(conn, builtin.id)? || skill_exists(conn, builtin.id, &SkillScope::Global, None)? {
            continue;
        }
        let metadata = builtin_metadata(builtin.clone());
        let content = compose_skill_md(&metadata, builtin.body);
        let skill_dir = skill_dir(&SkillScope::Global, None, builtin.id)?;
        write_skill_source(&skill_dir, &content)?;
        upsert_skill(
            conn,
            &SkillRecord {
                id: metadata.id.clone(),
                scope: SkillScope::Global,
                workspace_path: None,
                source: SkillSource::Builtin,
                enabled: true,
                skill_md_path: skill_dir.join(SKILL_FILE).to_string_lossy().to_string(),
                skill_dir: skill_dir.to_string_lossy().to_string(),
                content_hash: content_hash(&content),
                metadata,
                created_at: now_string(),
                updated_at: now_string(),
            },
        )?;
    }
    Ok(())
}

pub fn list_skills(conn: &Connection, input: SkillScopeInput) -> Result<SkillListResult, AppError> {
    seed_builtin_skills(conn)?;
    let workspace_key = workspace_key(&input.scope, input.workspace_path.as_deref())?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, scope, workspace_path, source, enabled, skill_dir, skill_md_path, content_hash,
               metadata_json, created_at, updated_at
        FROM skills
        WHERE scope = ?1 AND workspace_path = ?2
        ORDER BY source ASC, id ASC
        "#,
    )?;
    let rows = stmt.query_map(params![input.scope.as_str(), workspace_key], row_to_skill_record)?;
    let records = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;
    let mut skills = Vec::new();
    for record in records {
        skills.push(record_to_skill(conn, record)?);
    }
    let stats = SkillStats {
        total: skills.len(),
        enabled: skills.iter().filter(|skill| skill.enabled).count(),
        mounted: skills
            .iter()
            .filter(|skill| skill.bindings.iter().any(|binding| binding.mounted))
            .count(),
    };
    Ok(SkillListResult { skills, stats })
}

pub fn list_mount_paths(conn: &Connection) -> Result<Vec<SkillAgentMountPath>, AppError> {
    let mut result = Vec::new();
    let mut stmt = conn.prepare("SELECT id FROM agents ORDER BY id")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for agent_id in ids {
        let configured = conn
            .query_row(
                "SELECT mount_path FROM skill_agent_mount_paths WHERE agent_id = ?1",
                params![agent_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        result.push(SkillAgentMountPath {
            mount_path: configured
                .clone()
                .unwrap_or_else(|| default_mount_path(&agent_id).to_string()),
            is_default: configured.is_none(),
            agent_id,
        });
    }
    Ok(result)
}

pub fn update_mount_path(
    conn: &Connection,
    agent_id: &str,
    mount_path: &str,
) -> Result<SkillMountMigrationReport, AppError> {
    ensure_agent_exists(conn, agent_id)?;
    let old_mount_path = mount_path_for_agent(conn, agent_id)?;
    let now = now_string();
    conn.execute(
        r#"
        INSERT INTO skill_agent_mount_paths (agent_id, mount_path, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(agent_id) DO UPDATE SET mount_path = excluded.mount_path, updated_at = excluded.updated_at
        "#,
        params![agent_id, mount_path.trim(), now, now],
    )?;
    let mut report = SkillMountMigrationReport {
        agent_id: agent_id.to_string(),
        old_mount_path,
        new_mount_path: mount_path.trim().to_string(),
        migrated: Vec::new(),
        removed: Vec::new(),
        overwritten: Vec::new(),
        backed_up: Vec::new(),
        failed: Vec::new(),
    };
    let records = bound_skills_for_agent(conn, agent_id)?;
    for record in records {
        let source = PathBuf::from(&record.skill_dir);
        let old_target = scope_root(&record.scope, record.workspace_path.as_deref())?
            .join(&report.old_mount_path)
            .join(&record.id);
        if is_managed_link(&old_target, &source) {
            if let Err(error) = fs::remove_file(&old_target).or_else(|_| fs::remove_dir(&old_target)) {
                report.failed.push(SkillFailure {
                    skill_id: record.id.clone(),
                    reason: error.to_string(),
                });
                continue;
            }
            report.removed.push(old_target.to_string_lossy().to_string());
        }
        match mount_skill(&record, agent_id, mount_path, true) {
            Ok(mut mounted) => {
                report.migrated.push(record.id.clone());
                report.overwritten.append(&mut mounted.overwritten);
                report.backed_up.append(&mut mounted.backed_up);
            }
            Err(error) => report.failed.push(SkillFailure {
                skill_id: record.id,
                reason: error.to_string(),
            }),
        }
    }
    Ok(report)
}

pub fn create_skill(conn: &Connection, input: SkillMutationInput) -> Result<Skill, AppError> {
    validate_metadata(&input.metadata)?;
    if input.id != input.metadata.id {
        return Err(AppError::Validation("Skill id must match metadata id".to_string()));
    }
    if skill_exists(
        conn,
        &input.id,
        &input.scope,
        input.workspace_path.as_deref(),
    )? {
        return Err(AppError::Validation(format!(
            "Skill already exists: {}",
            input.id
        )));
    }
    let content = compose_skill_md(&input.metadata, &input.body);
    let skill_dir = skill_dir(&input.scope, input.workspace_path.as_deref(), &input.id)?;
    write_skill_source(&skill_dir, &content)?;
    let now = now_string();
    let record = SkillRecord {
        id: input.id,
        scope: input.scope,
        workspace_path: normalized_workspace(&input.workspace_path),
        source: input.source.unwrap_or(SkillSource::User),
        enabled: input.enabled,
        skill_md_path: skill_dir.join(SKILL_FILE).to_string_lossy().to_string(),
        skill_dir: skill_dir.to_string_lossy().to_string(),
        content_hash: content_hash(&content),
        metadata: input.metadata,
        created_at: now.clone(),
        updated_at: now,
    };
    upsert_skill(conn, &record)?;
    set_bindings(conn, &record, &input.bound_agent_ids)?;
    record_to_skill(conn, load_skill_record(conn, &record.id, &record.scope, record.workspace_path.as_deref())?)
}

pub fn update_skill(
    conn: &Connection,
    skill_id: &str,
    input: SkillUpdateInput,
) -> Result<Skill, AppError> {
    validate_metadata(&input.metadata)?;
    if input.metadata.id != skill_id {
        return Err(AppError::Validation("Skill id cannot be changed".to_string()));
    }
    let mut record = load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?;
    let content = compose_skill_md(&input.metadata, &input.body);
    write_skill_source(Path::new(&record.skill_dir), &content)?;
    record.metadata = input.metadata;
    record.enabled = input.enabled;
    record.content_hash = content_hash(&content);
    record.updated_at = now_string();
    upsert_skill(conn, &record)?;
    set_bindings(conn, &record, &input.bound_agent_ids)?;
    record_to_skill(conn, load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?)
}

pub fn delete_skill(
    conn: &Connection,
    skill_id: &str,
    input: SkillScopeInput,
) -> Result<(), AppError> {
    let record = load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?;
    for agent_id in binding_agent_ids(conn, &record)? {
        let target = mount_target(&record, &agent_id, &mount_path_for_agent(conn, &agent_id)?)?;
        if is_managed_link(&target, Path::new(&record.skill_dir)) {
            let _ = fs::remove_file(&target).or_else(|_| fs::remove_dir(&target));
        }
    }
    if record.source == SkillSource::Builtin {
        conn.execute(
            "INSERT OR REPLACE INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
            params![record.id.as_str(), now_string()],
        )?;
    }
    let workspace_key = workspace_key(&record.scope, record.workspace_path.as_deref())?;
    conn.execute(
        "DELETE FROM skill_agent_bindings WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3",
        params![record.id.as_str(), record.scope.as_str(), workspace_key.as_str()],
    )?;
    conn.execute(
        "DELETE FROM skills WHERE id = ?1 AND scope = ?2 AND workspace_path = ?3",
        params![record.id.as_str(), record.scope.as_str(), workspace_key.as_str()],
    )?;
    let _ = fs::remove_dir_all(record.skill_dir);
    Ok(())
}

pub fn restore_builtin(conn: &Connection, skill_id: &str) -> Result<Skill, AppError> {
    let builtin = BUILTIN_SKILLS
        .iter()
        .find(|skill| skill.id == skill_id)
        .ok_or_else(|| AppError::Validation(format!("Unknown built-in Skill: {skill_id}")))?;
    conn.execute(
        "DELETE FROM deleted_builtin_skills WHERE skill_id = ?1",
        params![skill_id],
    )?;
    let metadata = builtin_metadata(builtin.clone());
    let content = compose_skill_md(&metadata, builtin.body);
    let skill_dir = skill_dir(&SkillScope::Global, None, skill_id)?;
    write_skill_source(&skill_dir, &content)?;
    let now = now_string();
    let record = SkillRecord {
        id: skill_id.to_string(),
        scope: SkillScope::Global,
        workspace_path: None,
        source: SkillSource::Builtin,
        enabled: true,
        skill_md_path: skill_dir.join(SKILL_FILE).to_string_lossy().to_string(),
        skill_dir: skill_dir.to_string_lossy().to_string(),
        content_hash: content_hash(&content),
        metadata,
        created_at: now.clone(),
        updated_at: now,
    };
    upsert_skill(conn, &record)?;
    record_to_skill(conn, record)
}

pub fn set_skill_enabled(
    conn: &Connection,
    skill_id: &str,
    input: SkillScopeInput,
    enabled: bool,
) -> Result<Skill, AppError> {
    let mut record = load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?;
    record.enabled = enabled;
    record.updated_at = now_string();
    upsert_skill(conn, &record)?;
    let agents = binding_agent_ids(conn, &record)?;
    if enabled {
        for agent_id in &agents {
            let mount_path = mount_path_for_agent(conn, agent_id)?;
            let _ = mount_skill(&record, agent_id, &mount_path, true);
        }
    } else {
        for agent_id in &agents {
            let target = mount_target(&record, agent_id, &mount_path_for_agent(conn, agent_id)?)?;
            if is_managed_link(&target, Path::new(&record.skill_dir)) {
                let _ = fs::remove_file(&target).or_else(|_| fs::remove_dir(&target));
            }
        }
    }
    record_to_skill(conn, load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?)
}

pub fn set_skill_bindings(
    conn: &Connection,
    skill_id: &str,
    input: SkillScopeInput,
    agent_ids: Vec<String>,
) -> Result<Skill, AppError> {
    let record = load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?;
    set_bindings(conn, &record, &agent_ids)?;
    record_to_skill(conn, load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?)
}

pub fn preview_skill(
    conn: &Connection,
    skill_id: &str,
    input: SkillScopeInput,
) -> Result<SkillPreview, AppError> {
    let record = load_skill_record(conn, skill_id, &input.scope, input.workspace_path.as_deref())?;
    let content = fs::read_to_string(&record.skill_md_path)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(SkillPreview {
        id: skill_id.to_string(),
        scope: input.scope,
        workspace_path: normalized_workspace(&input.workspace_path),
        content,
        path: record.skill_md_path,
    })
}

pub fn import_skill(conn: &Connection, input: SkillImportInput) -> Result<Skill, AppError> {
    let source_dir = PathBuf::from(&input.source_path);
    let content = fs::read_to_string(source_dir.join(SKILL_FILE))
        .map_err(|error| AppError::Validation(format!("Invalid Skill source: {error}")))?;
    let metadata = parse_skill_md(&content)?;
    if skill_exists(
        conn,
        &metadata.id,
        &input.scope,
        input.workspace_path.as_deref(),
    )? {
        return Err(AppError::Validation(format!(
            "Skill already exists: {}",
            metadata.id
        )));
    }
    let target_dir = skill_dir(&input.scope, input.workspace_path.as_deref(), &metadata.id)?;
    copy_dir_all(&source_dir, &target_dir)?;
    let now = now_string();
    let record = SkillRecord {
        id: metadata.id.clone(),
        scope: input.scope,
        workspace_path: normalized_workspace(&input.workspace_path),
        source: SkillSource::Imported,
        enabled: input.enabled,
        skill_md_path: target_dir.join(SKILL_FILE).to_string_lossy().to_string(),
        skill_dir: target_dir.to_string_lossy().to_string(),
        content_hash: content_hash(&content),
        metadata,
        created_at: now.clone(),
        updated_at: now,
    };
    upsert_skill(conn, &record)?;
    set_bindings(conn, &record, &input.bound_agent_ids)?;
    record_to_skill(conn, record)
}

pub fn detect_drift(conn: &Connection, input: SkillScopeInput) -> Result<SkillDriftReport, AppError> {
    let list = list_skills(conn, input.clone())?;
    let source_root = source_root(&input.scope, input.workspace_path.as_deref())?;
    let mut issues = Vec::new();
    for skill in &list.skills {
        let skill_md = PathBuf::from(&skill.skill_md_path);
        if !skill_md.exists() {
            issues.push(SkillDriftIssue {
                skill_id: skill.id.clone(),
                r#type: SkillDriftIssueType::MissingSource,
                agent_id: None,
                path: Some(skill.skill_md_path.clone()),
                message: "SKILL.md is missing".to_string(),
            });
            continue;
        }
        let content = fs::read_to_string(&skill_md).map_err(|error| AppError::Storage(error.to_string()))?;
        if content_hash(&content) != skill.content_hash {
            issues.push(SkillDriftIssue {
                skill_id: skill.id.clone(),
                r#type: SkillDriftIssueType::MetadataChanged,
                agent_id: None,
                path: Some(skill.skill_md_path.clone()),
                message: "SKILL.md differs from the registry snapshot".to_string(),
            });
        }
        if skill.enabled {
            for binding in &skill.bindings {
                let target = PathBuf::from(&binding.mounted_path);
                if !target.exists() {
                    issues.push(SkillDriftIssue {
                        skill_id: skill.id.clone(),
                        r#type: SkillDriftIssueType::MissingMount,
                        agent_id: Some(binding.agent_id.clone()),
                        path: Some(binding.mounted_path.clone()),
                        message: "Agent mount is missing".to_string(),
                    });
                } else if !is_managed_link(&target, Path::new(&skill.skill_dir)) {
                    issues.push(SkillDriftIssue {
                        skill_id: skill.id.clone(),
                        r#type: SkillDriftIssueType::Conflict,
                        agent_id: Some(binding.agent_id.clone()),
                        path: Some(binding.mounted_path.clone()),
                        message: "Agent mount path is occupied by unmanaged content".to_string(),
                    });
                }
            }
        }
    }
    for entry in fs::read_dir(&source_root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.join(SKILL_FILE).is_file() {
            let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !skill_exists(conn, id, &input.scope, input.workspace_path.as_deref())? {
                issues.push(SkillDriftIssue {
                    skill_id: id.to_string(),
                    r#type: SkillDriftIssueType::UnregisteredSource,
                    agent_id: None,
                    path: Some(path.to_string_lossy().to_string()),
                    message: "Skill source exists without a registry record".to_string(),
                });
            }
        }
    }
    for builtin in BUILTIN_SKILLS {
        if input.scope == SkillScope::Global && is_builtin_deleted(conn, builtin.id)? {
            issues.push(SkillDriftIssue {
                skill_id: builtin.id.to_string(),
                r#type: SkillDriftIssueType::DeletedBuiltin,
                agent_id: None,
                path: None,
                message: "Built-in Skill is deleted and can be restored".to_string(),
            });
        }
    }
    let drift_hash = content_hash(&serde_json::to_string(&issues).unwrap_or_default());
    let report = SkillDriftReport {
        scope: input.scope,
        workspace_path: normalized_workspace(&input.workspace_path),
        issues,
        drift_hash,
    };
    save_drift_snapshot(conn, &report)?;
    Ok(report)
}

pub fn sync_drift(conn: &Connection, input: SkillScopeInput) -> Result<SkillSyncResult, AppError> {
    let report = detect_drift(conn, input.clone())?;
    let mut result = SkillSyncResult {
        mounted: Vec::new(),
        unmounted: Vec::new(),
        overwritten: Vec::new(),
        backed_up: Vec::new(),
        restored: Vec::new(),
        failed: Vec::new(),
        resolved_from: report.clone(),
    };
    for issue in &report.issues {
        match issue.r#type {
            SkillDriftIssueType::MissingMount | SkillDriftIssueType::Conflict => {
                let Some(agent_id) = issue.agent_id.as_deref() else {
                    continue;
                };
                match load_skill_record(conn, &issue.skill_id, &input.scope, input.workspace_path.as_deref())
                    .and_then(|record| {
                        let mount_path = mount_path_for_agent(conn, agent_id)?;
                        mount_skill(&record, agent_id, &mount_path, true)
                    }) {
                    Ok(mut mounted) => {
                        result.mounted.push(issue.skill_id.clone());
                        result.overwritten.append(&mut mounted.overwritten);
                        result.backed_up.append(&mut mounted.backed_up);
                    }
                    Err(error) => result.failed.push(SkillFailure {
                        skill_id: issue.skill_id.clone(),
                        reason: error.to_string(),
                    }),
                }
            }
            SkillDriftIssueType::MetadataChanged => {
                match fs::read_to_string(issue.path.as_deref().unwrap_or_default())
                    .map_err(|error| AppError::Storage(error.to_string()))
                    .and_then(|content| {
                        let metadata = parse_skill_md(&content)?;
                        let mut record = load_skill_record(
                            conn,
                            &issue.skill_id,
                            &input.scope,
                            input.workspace_path.as_deref(),
                        )?;
                        record.metadata = metadata;
                        record.content_hash = content_hash(&content);
                        record.updated_at = now_string();
                        upsert_skill(conn, &record)
                    }) {
                    Ok(()) => result.restored.push(issue.skill_id.clone()),
                    Err(error) => result.failed.push(SkillFailure {
                        skill_id: issue.skill_id.clone(),
                        reason: error.to_string(),
                    }),
                }
            }
            SkillDriftIssueType::DeletedBuiltin => {
                if let Err(error) = restore_builtin(conn, &issue.skill_id) {
                    result.failed.push(SkillFailure {
                        skill_id: issue.skill_id.clone(),
                        reason: error.to_string(),
                    });
                } else {
                    result.restored.push(issue.skill_id.clone());
                }
            }
            SkillDriftIssueType::MissingSource | SkillDriftIssueType::UnregisteredSource => {}
        }
    }
    Ok(result)
}

pub fn select_workspace_directory() -> Result<Option<String>, AppError> {
    let cwd = std::env::current_dir().map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(Some(cwd.to_string_lossy().to_string()))
}

fn row_to_skill_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SkillRecord> {
    let scope_raw = row.get::<_, String>(1)?;
    let source_raw = row.get::<_, String>(3)?;
    let metadata_json = row.get::<_, String>(8)?;
    let metadata = from_json::<SkillMetadata>(&metadata_json).map_err(json_to_sql_error)?;
    Ok(SkillRecord {
        id: row.get(0)?,
        scope: SkillScope::parse(&scope_raw).unwrap_or(SkillScope::Global),
        workspace_path: workspace_from_key(row.get::<_, String>(2)?),
        source: SkillSource::parse(&source_raw).unwrap_or(SkillSource::User),
        enabled: row.get::<_, i32>(4)? != 0,
        skill_dir: row.get(5)?,
        skill_md_path: row.get(6)?,
        content_hash: row.get(7)?,
        metadata,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn record_to_skill(conn: &Connection, record: SkillRecord) -> Result<Skill, AppError> {
    let agent_ids = binding_agent_ids(conn, &record)?;
    let mut bindings = Vec::new();
    for agent_id in &agent_ids {
        let mount_path = mount_path_for_agent(conn, agent_id)?;
        let mounted_path = mount_target(&record, agent_id, &mount_path)?;
        bindings.push(SkillAgentBinding {
            agent_id: agent_id.clone(),
            mount_path,
            mounted: is_managed_link(&mounted_path, Path::new(&record.skill_dir)),
            mounted_path: mounted_path.to_string_lossy().to_string(),
        });
    }
    Ok(Skill {
        id: record.id,
        scope: record.scope,
        workspace_path: record.workspace_path,
        source: record.source,
        enabled: record.enabled,
        skill_dir: record.skill_dir,
        skill_md_path: record.skill_md_path,
        content_hash: record.content_hash,
        metadata: record.metadata,
        bound_agent_ids: agent_ids,
        bindings,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn load_skill_record(
    conn: &Connection,
    skill_id: &str,
    scope: &SkillScope,
    workspace_path: Option<&str>,
) -> Result<SkillRecord, AppError> {
    let workspace_key = workspace_key(scope, workspace_path)?;
    conn.query_row(
        r#"
        SELECT id, scope, workspace_path, source, enabled, skill_dir, skill_md_path, content_hash,
               metadata_json, created_at, updated_at
        FROM skills
        WHERE id = ?1 AND scope = ?2 AND workspace_path = ?3
        "#,
        params![skill_id, scope.as_str(), workspace_key],
        row_to_skill_record,
    )
    .optional()?
    .ok_or_else(|| AppError::Validation(format!("Skill not found: {skill_id}")))
}

fn upsert_skill(conn: &Connection, record: &SkillRecord) -> Result<(), AppError> {
    let workspace_key = workspace_key(&record.scope, record.workspace_path.as_deref())?;
    conn.execute(
        r#"
        INSERT INTO skills
        (id, scope, workspace_path, source, enabled, skill_dir, skill_md_path, content_hash,
         metadata_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id, scope, workspace_path) DO UPDATE SET
            source = excluded.source,
            enabled = excluded.enabled,
            skill_dir = excluded.skill_dir,
            skill_md_path = excluded.skill_md_path,
            content_hash = excluded.content_hash,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at
        "#,
        params![
            record.id.as_str(),
            record.scope.as_str(),
            workspace_key.as_str(),
            record.source.as_str(),
            record.enabled as i32,
            record.skill_dir.as_str(),
            record.skill_md_path.as_str(),
            record.content_hash.as_str(),
            to_json(&record.metadata)?,
            record.created_at.as_str(),
            record.updated_at.as_str(),
        ],
    )?;
    Ok(())
}

fn set_bindings(conn: &Connection, record: &SkillRecord, agent_ids: &[String]) -> Result<(), AppError> {
    let current = binding_agent_ids(conn, record)?;
    let workspace_key = workspace_key(&record.scope, record.workspace_path.as_deref())?;
    for agent_id in current.iter().filter(|agent_id| !agent_ids.contains(agent_id)) {
        let target = mount_target(record, agent_id, &mount_path_for_agent(conn, agent_id)?)?;
        if is_managed_link(&target, Path::new(&record.skill_dir)) {
            let _ = fs::remove_file(&target).or_else(|_| fs::remove_dir(&target));
        }
        conn.execute(
            "DELETE FROM skill_agent_bindings WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3 AND agent_id = ?4",
            params![record.id.as_str(), record.scope.as_str(), workspace_key.as_str(), agent_id.as_str()],
        )?;
    }
    for agent_id in agent_ids {
        ensure_agent_exists(conn, agent_id)?;
        let mount_path = mount_path_for_agent(conn, agent_id)?;
        let target = mount_target(record, agent_id, &mount_path)?;
        let now = now_string();
        conn.execute(
            r#"
            INSERT INTO skill_agent_bindings
            (skill_id, scope, workspace_path, agent_id, mounted_path, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(skill_id, scope, workspace_path, agent_id) DO UPDATE SET
                mounted_path = excluded.mounted_path,
                status = excluded.status,
                updated_at = excluded.updated_at
            "#,
            params![
                record.id.as_str(),
                record.scope.as_str(),
                workspace_key.as_str(),
                agent_id.as_str(),
                target.to_string_lossy().to_string(),
                if record.enabled { "mounted" } else { "disabled" },
                now.as_str(),
                now.as_str(),
            ],
        )?;
        if record.enabled {
            let _ = mount_skill(record, agent_id, &mount_path, true);
        }
    }
    Ok(())
}

fn binding_agent_ids(conn: &Connection, record: &SkillRecord) -> Result<Vec<String>, AppError> {
    let workspace_key = workspace_key(&record.scope, record.workspace_path.as_deref())?;
    let mut stmt = conn.prepare(
        r#"
        SELECT agent_id FROM skill_agent_bindings
        WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3
        ORDER BY agent_id
        "#,
    )?;
    let rows = stmt.query_map(
        params![record.id.as_str(), record.scope.as_str(), workspace_key],
        |row| row.get::<_, String>(0),
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

fn bound_skills_for_agent(conn: &Connection, agent_id: &str) -> Result<Vec<SkillRecord>, AppError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT s.id, s.scope, s.workspace_path, s.source, s.enabled, s.skill_dir, s.skill_md_path,
               s.content_hash, s.metadata_json, s.created_at, s.updated_at
        FROM skills s
        INNER JOIN skill_agent_bindings b
          ON s.id = b.skill_id AND s.scope = b.scope AND s.workspace_path = b.workspace_path
        WHERE b.agent_id = ?1 AND s.enabled = 1
        "#,
    )?;
    let rows = stmt.query_map(params![agent_id], row_to_skill_record)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)
}

#[derive(Default)]
struct MountOutcome {
    overwritten: Vec<String>,
    backed_up: Vec<SkillBackupEntry>,
}

fn mount_skill(
    record: &SkillRecord,
    agent_id: &str,
    mount_path: &str,
    overwrite: bool,
) -> Result<MountOutcome, AppError> {
    let source = PathBuf::from(&record.skill_dir);
    let target = mount_target(record, agent_id, mount_path)?;
    let mut outcome = MountOutcome::default();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    if target.exists() || fs::symlink_metadata(&target).is_ok() {
        if is_managed_link(&target, &source) {
            return Ok(outcome);
        }
        if !overwrite {
            return Err(AppError::Storage(format!(
                "Mount target already exists: {}",
                target.to_string_lossy()
            )));
        }
        let backup = backup_path(record, &target)?;
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent).map_err(|error| AppError::Storage(error.to_string()))?;
        }
        fs::rename(&target, &backup).map_err(|error| AppError::Storage(error.to_string()))?;
        outcome.overwritten.push(target.to_string_lossy().to_string());
        outcome.backed_up.push(SkillBackupEntry {
            original_path: target.to_string_lossy().to_string(),
            backup_path: backup.to_string_lossy().to_string(),
        });
    }
    create_dir_link(&source, &target)?;
    Ok(outcome)
}

fn create_dir_link(source: &Path, target: &Path) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        match std::os::windows::fs::symlink_dir(source, target) {
            Ok(()) => Ok(()),
            Err(_) => {
                let status = std::process::Command::new("cmd")
                    .args([
                        "/C",
                        "mklink",
                        "/J",
                        &target.to_string_lossy(),
                        &source.to_string_lossy(),
                    ])
                    .status()
                    .map_err(|error| AppError::Storage(error.to_string()))?;
                if status.success() {
                    Ok(())
                } else {
                    Err(AppError::Storage(format!(
                        "Failed to create directory link: {}",
                        target.to_string_lossy()
                    )))
                }
            }
        }
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).map_err(|error| AppError::Storage(error.to_string()))
    }
}

fn is_managed_link(target: &Path, source: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(target) else {
        return false;
    };
    if !metadata.file_type().is_symlink() {
        return false;
    }
    let Ok(link_target) = fs::read_link(target) else {
        return false;
    };
    paths_equal(&link_target, source)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
    } else {
        left == right
    }
}

fn mount_target(record: &SkillRecord, _agent_id: &str, mount_path: &str) -> Result<PathBuf, AppError> {
    Ok(scope_root(&record.scope, record.workspace_path.as_deref())?
        .join(mount_path)
        .join(&record.id))
}

fn backup_path(record: &SkillRecord, target: &Path) -> Result<PathBuf, AppError> {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(record.id.as_str());
    Ok(scope_root(&record.scope, record.workspace_path.as_deref())?
        .join(VANEHUB_DIR)
        .join("backups")
        .join(SKILLS_DIR)
        .join(now_string())
        .join(name))
}

fn skill_dir(scope: &SkillScope, workspace_path: Option<&str>, skill_id: &str) -> Result<PathBuf, AppError> {
    Ok(source_root(scope, workspace_path)?.join(skill_id))
}

fn source_root(scope: &SkillScope, workspace_path: Option<&str>) -> Result<PathBuf, AppError> {
    Ok(match scope {
        SkillScope::Global => home_dir()?.join(VANEHUB_DIR).join(SKILLS_DIR),
        SkillScope::Workspace => workspace_root(workspace_path)?.join(VANEHUB_DIR).join(SKILLS_DIR),
    })
}

fn scope_root(scope: &SkillScope, workspace_path: Option<&str>) -> Result<PathBuf, AppError> {
    Ok(match scope {
        SkillScope::Global => home_dir()?,
        SkillScope::Workspace => workspace_root(workspace_path)?,
    })
}

fn workspace_root(workspace_path: Option<&str>) -> Result<PathBuf, AppError> {
    let path = workspace_path
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| AppError::Validation("Workspace path is required".to_string()))?;
    Ok(PathBuf::from(path))
}

fn home_dir() -> Result<PathBuf, AppError> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| AppError::Storage("Unable to resolve home directory".to_string()))
}

fn write_skill_source(skill_dir: &Path, content: &str) -> Result<(), AppError> {
    fs::create_dir_all(skill_dir).map_err(|error| AppError::Storage(error.to_string()))?;
    fs::write(skill_dir.join(SKILL_FILE), content).map_err(|error| AppError::Storage(error.to_string()))
}

fn parse_skill_md(content: &str) -> Result<SkillMetadata, AppError> {
    let frontmatter = content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(frontmatter, _)| frontmatter)
        .ok_or_else(|| AppError::Validation("SKILL.md requires frontmatter".to_string()))?;
    let mut id = String::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut category = String::new();
    let mut version = String::new();
    let mut triggers = Vec::new();
    let mut in_triggers = false;
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if in_triggers && line.starts_with('-') {
            triggers.push(line.trim_start_matches('-').trim().trim_matches('"').to_string());
            continue;
        }
        in_triggers = false;
        if line == "triggers:" {
            in_triggers = true;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "id" => id = value,
            "name" => name = value,
            "description" => description = value,
            "category" => category = value,
            "version" => version = value,
            _ => {}
        }
    }
    let metadata = SkillMetadata {
        id,
        name,
        description,
        category,
        version,
        triggers,
    };
    validate_metadata(&metadata)?;
    Ok(metadata)
}

fn compose_skill_md(metadata: &SkillMetadata, body: &str) -> String {
    let triggers = metadata
        .triggers
        .iter()
        .map(|trigger| format!("  - {trigger}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nid: {}\nname: {}\ndescription: {}\ncategory: {}\nversion: {}\ntriggers:\n{}\n---\n\n# {}\n\n{}\n",
        metadata.id,
        metadata.name,
        metadata.description,
        metadata.category,
        metadata.version,
        triggers,
        metadata.name,
        body.trim()
    )
}

fn validate_metadata(metadata: &SkillMetadata) -> Result<(), AppError> {
    if !is_kebab_case(&metadata.id) {
        return Err(AppError::Validation(
            "Skill id must be kebab-case letters, digits, and hyphens".to_string(),
        ));
    }
    if metadata.name.trim().is_empty()
        || metadata.description.trim().is_empty()
        || metadata.category.trim().is_empty()
        || metadata.version.trim().is_empty()
    {
        return Err(AppError::Validation(
            "Skill metadata requires name, description, category, and version".to_string(),
        ));
    }
    Ok(())
}

fn builtin_metadata(skill: BuiltinSkill) -> SkillMetadata {
    SkillMetadata {
        id: skill.id.to_string(),
        name: skill.name.to_string(),
        description: skill.description.to_string(),
        category: skill.category.to_string(),
        version: "1.0.0".to_string(),
        triggers: skill.triggers.iter().map(|trigger| trigger.to_string()).collect(),
    }
}

fn skill_exists(
    conn: &Connection,
    skill_id: &str,
    scope: &SkillScope,
    workspace_path: Option<&str>,
) -> Result<bool, AppError> {
    let workspace_key = workspace_key(scope, workspace_path)?;
    conn.query_row(
        "SELECT 1 FROM skills WHERE id = ?1 AND scope = ?2 AND workspace_path = ?3",
        params![skill_id, scope.as_str(), workspace_key],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(AppError::Database)
}

fn is_builtin_deleted(conn: &Connection, skill_id: &str) -> Result<bool, AppError> {
    conn.query_row(
        "SELECT 1 FROM deleted_builtin_skills WHERE skill_id = ?1",
        params![skill_id],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(AppError::Database)
}

fn mount_path_for_agent(conn: &Connection, agent_id: &str) -> Result<String, AppError> {
    Ok(conn
        .query_row(
            "SELECT mount_path FROM skill_agent_mount_paths WHERE agent_id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| default_mount_path(agent_id).to_string()))
}

fn ensure_agent_exists(conn: &Connection, agent_id: &str) -> Result<(), AppError> {
    let exists = conn
        .query_row("SELECT 1 FROM agents WHERE id = ?1", params![agent_id], |_| Ok(()))
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(AppError::AgentNotFound(agent_id.to_string()))
    }
}

fn default_mount_path(agent_id: &str) -> &'static str {
    match agent_id {
        "claude-code" => ".claude/skills",
        "codex-cli" => ".codex/skills",
        "gemini-cli" => ".gemini/skills",
        "opencode" => ".opencode/skills",
        _ => ".vanehub/skills",
    }
}

fn save_drift_snapshot(conn: &Connection, report: &SkillDriftReport) -> Result<(), AppError> {
    let workspace_key = workspace_key(&report.scope, report.workspace_path.as_deref())?;
    let now = now_string();
    conn.execute(
        r#"
        INSERT INTO skill_drift_snapshots (scope, workspace_path, drift_hash, issues_json, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(scope, workspace_path) DO UPDATE SET
            drift_hash = excluded.drift_hash,
            issues_json = excluded.issues_json,
            updated_at = excluded.updated_at
        "#,
        params![
            report.scope.as_str(),
            workspace_key,
            report.drift_hash.as_str(),
            to_json(&report.issues)?,
            now,
        ],
    )?;
    Ok(())
}

fn copy_dir_all(source: &Path, target: &Path) -> Result<(), AppError> {
    fs::create_dir_all(target).map_err(|error| AppError::Storage(error.to_string()))?;
    for entry in fs::read_dir(source).map_err(|error| AppError::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| AppError::Storage(error.to_string()))?;
        let file_type = entry.file_type().map_err(|error| AppError::Storage(error.to_string()))?;
        let dest = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest).map_err(|error| AppError::Storage(error.to_string()))?;
        }
    }
    Ok(())
}

fn workspace_key(scope: &SkillScope, workspace_path: Option<&str>) -> Result<String, AppError> {
    match scope {
        SkillScope::Global => Ok(String::new()),
        SkillScope::Workspace => workspace_path
            .filter(|path| !path.trim().is_empty())
            .map(|path| path.trim().to_string())
            .ok_or_else(|| AppError::Validation("Workspace path is required".to_string())),
    }
}

fn workspace_from_key(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn normalized_workspace(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn is_kebab_case(value: &str) -> bool {
    if value.is_empty() || value.starts_with('-') || value.ends_with('-') {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn to_json<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|error| AppError::Validation(error.to_string()))
}

fn from_json<T: DeserializeOwned>(value: &str) -> Result<T, AppError> {
    serde_json::from_str(value).map_err(|error| AppError::Validation(error.to_string()))
}

fn json_to_sql_error(error: AppError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn now_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("sqlite");
        conn.execute_batch(
            r#"
            CREATE TABLE agents (id TEXT PRIMARY KEY);
            INSERT INTO agents (id) VALUES ('claude-code'), ('codex-cli'), ('gemini-cli'), ('opencode');
            "#,
        )
        .expect("agents");
        apply_schema(&conn).expect("schema");
        conn
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("vanehub-skill-{name}-{unique}"))
    }

    #[test]
    fn parses_skill_frontmatter() {
        let content = "---\nid: sample-skill\nname: Sample\ndescription: Demo\ncategory: test\nversion: 1.0.0\ntriggers:\n  - demo\n---\n\n# Sample";
        let meta = parse_skill_md(content).expect("metadata");
        assert_eq!(meta.id, "sample-skill");
        assert_eq!(meta.triggers, vec!["demo"]);
    }

    #[test]
    fn seed_builtin_skills_is_idempotent() {
        let conn = conn();
        seed_builtin_skills(&conn).expect("first seed");
        seed_builtin_skills(&conn).expect("second seed");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skills WHERE source = 'builtin'", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 6);
    }

    #[test]
    fn immutable_id_is_rejected() {
        let metadata = SkillMetadata {
            id: "bad_name".to_string(),
            name: "Name".to_string(),
            description: "Desc".to_string(),
            category: "test".to_string(),
            version: "1.0.0".to_string(),
            triggers: vec![],
        };
        assert!(validate_metadata(&metadata).is_err());
    }

    #[test]
    fn restore_builtin_clears_deleted_marker() {
        let conn = conn();
        conn.execute(
            "INSERT INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
            params!["code-review", now_string()],
        )
        .expect("deleted marker");

        let restored = restore_builtin(&conn, "code-review").expect("restore");

        assert_eq!(restored.id, "code-review");
        assert!(!is_builtin_deleted(&conn, "code-review").expect("marker cleared"));
    }

    #[test]
    fn detects_missing_skill_source() {
        let conn = conn();
        let root = unique_temp_dir("missing-source");
        let metadata = SkillMetadata {
            id: "workspace-helper".to_string(),
            name: "Workspace Helper".to_string(),
            description: "test".to_string(),
            category: "testing".to_string(),
            version: "1.0.0".to_string(),
            triggers: vec![],
        };
        let skill = create_skill(
            &conn,
            SkillMutationInput {
                id: metadata.id.clone(),
                scope: SkillScope::Workspace,
                workspace_path: Some(root.to_string_lossy().to_string()),
                metadata,
                body: "body".to_string(),
                enabled: true,
                bound_agent_ids: vec![],
                source: Some(SkillSource::User),
            },
        )
        .expect("create");
        fs::remove_file(skill.skill_md_path).expect("remove source");

        let drift = detect_drift(
            &conn,
            SkillScopeInput {
                scope: SkillScope::Workspace,
                workspace_path: Some(root.to_string_lossy().to_string()),
            },
        )
        .expect("drift");

        assert!(drift
            .issues
            .iter()
            .any(|issue| issue.r#type == SkillDriftIssueType::MissingSource));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mount_path_migration_reports_agent_without_bound_skills() {
        let conn = conn();
        let report = update_mount_path(&conn, "codex-cli", ".codex/custom-skills").expect("migrate");

        assert_eq!(report.agent_id, "codex-cli");
        assert_eq!(report.old_mount_path, ".codex/skills");
        assert_eq!(report.new_mount_path, ".codex/custom-skills");
        assert!(report.failed.is_empty());
    }

    #[test]
    fn sync_restores_deleted_builtin_issue() {
        let conn = conn();
        conn.execute(
            "INSERT INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
            params!["readme-generation", now_string()],
        )
        .expect("deleted marker");

        let result = sync_drift(
            &conn,
            SkillScopeInput {
                scope: SkillScope::Global,
                workspace_path: None,
            },
        )
        .expect("sync");

        assert!(result.restored.contains(&"readme-generation".to_string()));
    }
}
