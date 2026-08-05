//! `grep` 与 `glob` 共用的工作区受限遍历。边界（路径越界、符号链接、取消、上限）只在这里
//! 实现一次，两个工具都不重复处理。

use crate::platform::filesystem::BoundedFilesystem;
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub(crate) const MAX_SEARCH_RESULTS: usize = 200;
pub(crate) const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;

/// 单文件读取上限。输出上限保护的是模型的上下文窗口，这一条保护的是进程本身 —— 没有它，
/// 一个未被 `.gitignore` 排除的大日志会在任何输出截断生效前就先分配等量内存。
pub(crate) const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// 判定二进制的嗅探窗口。整文件扫描对大文件不划算，而文本文件的 NUL 字节几乎总在开头出现。
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// 访问者对每个文件的处置：继续遍历，或提前终止（用于结果条数/字节到顶）。
pub(crate) enum Visit {
    Continue,
    Stop,
}

/// 遍历 `boundary` 根下（可选 `relative_root` 子目录）的常规文件，对每个文件调用 `visit`。
///
/// 符号链接一律跳过而非跟随后校验：跟随需要对每个条目做 canonicalize 系统调用，大仓库上代价
/// 显著；直接跳过可消除整类越界读取（例如仓库内指向 `~/.ssh/` 的链接）。
///
/// `require_git(false)` 是刻意的 —— 工作区未必是 git 仓库，但其中的 `.gitignore` 依然表达了
/// 「这些内容不值得看」，默认的 `require_git(true)` 会让非仓库工作区退化成搜索全部内容。
pub(crate) fn visit_workspace_files(
    workspace_folder: &str,
    relative_root: Option<&str>,
    cancelled: &Arc<AtomicBool>,
    visit: &mut dyn FnMut(&Path, &str) -> Visit,
) -> Result<(), String> {
    let boundary = BoundedFilesystem::new(Path::new(workspace_folder))
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    // 直接 canonicalize 而不走 `boundary.resolve_existing(".")` —— 后者依赖
    // `validate_relative` 接受 `Component::CurDir`，那是个未经验证的假设。
    let workspace_root = Path::new(workspace_folder)
        .canonicalize()
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    let root = match relative_root.map(str::trim).filter(|root| !root.is_empty()) {
        Some(relative) => boundary
            .resolve_existing(relative)
            .map_err(|error| format!("Path \"{relative}\" is not accessible: {error}"))?,
        None => workspace_root.clone(),
    };

    let walker = WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .build();

    for entry in walker {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Search was cancelled.".to_string());
        }
        let Ok(entry) = entry else {
            // 单个条目不可读（权限、竞态删除）不应让整次搜索失败。
            continue;
        };
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() || file_type.is_symlink() {
            continue;
        }
        let absolute = entry.path();
        let Ok(relative) = absolute.strip_prefix(&workspace_root) else {
            continue;
        };
        let display = relative.to_string_lossy().replace('\\', "/");
        if let Visit::Stop = visit(absolute, &display) {
            return Ok(());
        }
    }
    Ok(())
}

/// 二进制判定：嗅探窗口内出现 NUL 字节即认定为二进制。比向模型抛一个 UTF-8 解码错误
/// 更可解释 —— 模型据此知道该换一个文件，而不是重试同一个。
pub(crate) fn is_binary(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .take(BINARY_SNIFF_BYTES)
        .any(|byte| *byte == 0)
}

/// 在 `std::fs::read` 之前判断文件是否超过 `MAX_FILE_BYTES`。拿不到 metadata 时返回 `true`
/// —— 失败方向选择「不读」而非「读一个大小未知的文件」。
pub(crate) fn exceeds_size_limit(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() > MAX_FILE_BYTES,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn collect(directory: &TempDirectory, root: Option<&str>) -> Vec<String> {
        let folder = directory.path().to_string_lossy().to_string();
        let mut seen = Vec::new();
        visit_workspace_files(&folder, root, &not_cancelled(), &mut |_absolute, relative| {
            seen.push(relative.to_string());
            Visit::Continue
        })
        .expect("walk succeeds");
        seen.sort();
        seen
    }

    #[test]
    fn visits_plain_files_under_the_workspace() {
        let directory = TempDirectory::new("walk-plain");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/b.txt"), "b").expect("write b");
        assert_eq!(collect(&directory, None), vec!["a.txt", "sub/b.txt"]);
    }

    #[test]
    fn skips_gitignored_paths_even_outside_a_git_repository() {
        let directory = TempDirectory::new("walk-gitignore");
        std::fs::write(directory.path().join(".gitignore"), "ignored.txt\nnode_modules/\n")
            .expect("write gitignore");
        std::fs::write(directory.path().join("kept.txt"), "keep").expect("write kept");
        std::fs::write(directory.path().join("ignored.txt"), "drop").expect("write ignored");
        std::fs::create_dir(directory.path().join("node_modules")).expect("mkdir node_modules");
        std::fs::write(directory.path().join("node_modules/pkg.js"), "drop").expect("write pkg");
        let seen = collect(&directory, None);
        assert!(seen.contains(&"kept.txt".to_string()));
        assert!(!seen.iter().any(|path| path.contains("ignored.txt")));
        assert!(!seen.iter().any(|path| path.contains("node_modules")));
    }

    #[test]
    fn skips_hidden_entries() {
        let directory = TempDirectory::new("walk-hidden");
        std::fs::write(directory.path().join("visible.txt"), "v").expect("write visible");
        std::fs::create_dir(directory.path().join(".secret")).expect("mkdir .secret");
        std::fs::write(directory.path().join(".secret/key.txt"), "k").expect("write key");
        let seen = collect(&directory, None);
        assert_eq!(seen, vec!["visible.txt"]);
    }

    #[test]
    fn a_relative_root_narrows_the_walk() {
        let directory = TempDirectory::new("walk-root");
        std::fs::write(directory.path().join("top.txt"), "t").expect("write top");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/inner.txt"), "i").expect("write inner");
        assert_eq!(collect(&directory, Some("sub")), vec!["sub/inner.txt"]);
    }

    #[test]
    fn a_relative_root_that_escapes_the_workspace_is_rejected() {
        let directory = TempDirectory::new("walk-escape-root");
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            Some("../"),
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_cancelled_walk_stops_and_reports_an_error() {
        let directory = TempDirectory::new("walk-cancel");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        let cancelled = Arc::new(AtomicBool::new(true));
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &cancelled,
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = visit_workspace_files(
            "Z:/definitely/does/not/exist",
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_visitor_returning_stop_ends_the_walk_early() {
        let directory = TempDirectory::new("walk-stop");
        for index in 0..10 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "x")
                .expect("write fixture");
        }
        let mut count = 0usize;
        visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| {
                count += 1;
                Visit::Stop
            },
        )
        .expect("walk succeeds");
        assert_eq!(count, 1);
    }

    // 符号链接在 Windows 上需要开发者模式或管理员权限才能创建，故仅在 unix 下验证。
    // 跳过符号链接的逻辑本身与平台无关。
    #[cfg(unix)]
    #[test]
    fn a_symlink_pointing_outside_the_workspace_is_not_visited() {
        let outside = TempDirectory::new("walk-symlink-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            directory.path().join("leak.txt"),
        )
        .expect("create symlink");
        assert_eq!(collect(&directory, None), vec!["normal.txt"]);
    }

    #[test]
    fn binary_content_is_detected_by_a_nul_byte() {
        assert!(is_binary(b"abc\0def"));
        assert!(!is_binary(b"plain text"));
        assert!(!is_binary(b""));
    }

    #[test]
    fn a_small_file_is_within_the_size_limit() {
        let directory = TempDirectory::new("walk-size-small");
        let path = directory.path().join("small.txt");
        std::fs::write(&path, "tiny").expect("write fixture");
        assert!(!exceeds_size_limit(&path));
    }

    #[test]
    fn a_file_over_the_size_limit_is_rejected() {
        let directory = TempDirectory::new("walk-size-large");
        let path = directory.path().join("large.bin");
        std::fs::write(&path, vec![b'x'; (MAX_FILE_BYTES + 1) as usize]).expect("write fixture");
        assert!(exceeds_size_limit(&path));
    }

    #[test]
    fn an_unreadable_path_is_treated_as_over_the_limit() {
        // 拿不到 metadata 时保守判定为超限：调用方会跳过或报错，而不是继续去 read 一个
        // 大小未知的文件。
        assert!(exceeds_size_limit(Path::new("Z:/definitely/does/not/exist")));
    }
}
