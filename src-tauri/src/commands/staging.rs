use super::helpers::{delta_status_str, run_git_cancellable};
use super::types::{DiffLine, FileChange, FileDiff, HunkDiff};
use super::AppState;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn get_changed_files(path: String) -> Result<Vec<FileChange>, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let mut files = Vec::new();

    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;

    if let Some(tree) = head_tree {
        if let Ok(diff) = repo.diff_tree_to_index(Some(&tree), Some(&index), None) {
            for delta in diff.deltas() {
                let new_path = delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());
                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());
                files.push(FileChange {
                    path: new_path.unwrap_or_else(|| old_path.clone().unwrap_or_default()),
                    old_path: if delta.status() == git2::Delta::Renamed {
                        old_path
                    } else {
                        None
                    },
                    status: delta_status_str(delta.status()),
                    staged: true,
                });
            }
        }
    }

    {
        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        if let Ok(diff) = repo.diff_index_to_workdir(None, Some(&mut opts)) {
            for delta in diff.deltas() {
                let new_path = delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());
                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());
                files.push(FileChange {
                    path: new_path.unwrap_or_else(|| old_path.clone().unwrap_or_default()),
                    old_path: if delta.status() == git2::Delta::Renamed {
                        old_path
                    } else {
                        None
                    },
                    status: delta_status_str(delta.status()),
                    staged: false,
                });
            }
        }
    }

    Ok(files)
}

#[tauri::command]
pub fn get_file_diff(path: String, file_path: String, staged: bool) -> Result<FileDiff, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let diff = if staged {
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        let index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;

        let mut opts = git2::DiffOptions::new();
        opts.pathspec(&file_path);

        match head_tree {
            Some(tree) => repo
                .diff_tree_to_index(Some(&tree), Some(&index), Some(&mut opts))
                .map_err(|e| format!("Failed to get staged diff: {}", e))?,
            None => repo
                .diff_tree_to_index(None::<&git2::Tree>, Some(&index), Some(&mut opts))
                .map_err(|e| format!("Failed to get staged diff: {}", e))?,
        }
    } else {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(&file_path);
        opts.include_untracked(true).recurse_untracked_dirs(true);

        repo.diff_index_to_workdir(None, Some(&mut opts))
            .map_err(|e| format!("Failed to get unstaged diff: {}", e))?
    };

    let old_path = diff
        .deltas()
        .next()
        .and_then(|d| d.old_file().path())
        .map(|p| p.to_string_lossy().to_string());
    let new_path = diff
        .deltas()
        .next()
        .and_then(|d| d.new_file().path())
        .map(|p| p.to_string_lossy().to_string());

    let hunks = std::cell::RefCell::new(Vec::<HunkDiff>::new());
    let current_lines = std::cell::RefCell::new(Vec::<DiffLine>::new());
    let current_hunk_header = std::cell::RefCell::new(Option::<(u32, u32, u32, u32)>::None);

    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        Some(&mut |_delta, hunk| {
            if let Some((old_start, new_start, old_lines, new_lines)) =
                current_hunk_header.borrow_mut().take()
            {
                hunks.borrow_mut().push(HunkDiff {
                    old_start,
                    new_start,
                    old_lines,
                    new_lines,
                    lines: current_lines.borrow_mut().drain(..).collect(),
                });
            }
            *current_hunk_header.borrow_mut() = Some((
                hunk.old_start(),
                hunk.new_start(),
                hunk.old_lines(),
                hunk.new_lines(),
            ));
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let origin = match line.origin() {
                '+' => "add".to_string(),
                '-' => "remove".to_string(),
                _ => "context".to_string(),
            };
            current_lines.borrow_mut().push(DiffLine {
                old_line_no: line.old_lineno().map(|n| n as i32).unwrap_or(-1),
                new_line_no: line.new_lineno().map(|n| n as i32).unwrap_or(-1),
                content: String::from_utf8_lossy(line.content()).to_string(),
                origin,
            });
            true
        }),
    )
    .map_err(|e| format!("Failed to iterate diff: {}", e))?;

    if let Some((old_start, new_start, old_lines, new_lines)) =
        current_hunk_header.borrow_mut().take()
    {
        hunks.borrow_mut().push(HunkDiff {
            old_start,
            new_start,
            old_lines,
            new_lines,
            lines: current_lines.borrow_mut().drain(..).collect(),
        });
    }

    Ok(FileDiff {
        old_path,
        new_path,
        hunks: hunks.into_inner(),
    })
}

#[tauri::command]
pub fn stage_file(path: String, file_path: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    index
        .add_path(Path::new(&file_path))
        .map_err(|e| format!("Failed to stage file: {}", e))?;
    index
        .write()
        .map_err(|e| format!("Failed to write index: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn unstage_file(path: String, file_path: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let head_obj = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let head_object = head_obj.as_ref().map(|c| c.as_object());

    repo.reset_default(head_object, &[Path::new(&file_path)])
        .map_err(|e| format!("Failed to unstage file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn stage_all(path: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    index
        .add_all(&["."], git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| format!("Failed to stage all: {}", e))?;
    index
        .write()
        .map_err(|e| format!("Failed to write index: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn unstage_all(path: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let head_object = head_commit.as_ref().map(|c| c.as_object());

    let index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    let paths: Vec<PathBuf> = index
        .iter()
        .map(|e| String::from_utf8_lossy(&e.path).into_owned())
        .map(PathBuf::from)
        .collect();

    repo.reset_default(head_object, &paths)
        .map_err(|e| format!("Failed to unstage all: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<String, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    let tree_oid = index
        .write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo
        .find_tree(tree_oid)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    // L2: error on missing identity instead of using a fallback
    let sig = repo.signature().map_err(|_| {
        "No git identity configured. Run: git config --global user.name \"Your Name\" && git config --global user.email \"you@example.com\"".to_string()
    })?;

    let head = repo.head();
    let parent_commits: Vec<git2::Commit> = match head {
        Ok(ref h) => {
            let oid = h.target().ok_or("No HEAD target")?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| format!("Failed to find HEAD commit: {}", e))?;
            vec![commit]
        }
        Err(_) => vec![],
    };

    let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

    let commit_oid = if parent_refs.is_empty() {
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parent_refs)
    }
    .map_err(|e| format!("Failed to commit: {}", e))?;

    Ok(commit_oid.to_string())
}

#[tauri::command]
pub fn discard_file(
    path: String,
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    run_git_cancellable(&path, &["checkout", "HEAD", "--", &file_path], &state)?;
    Ok(())
}
