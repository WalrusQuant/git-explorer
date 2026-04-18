use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

pub struct AppState {
    pub git_op_pid: Arc<Mutex<Option<u32>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetail {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
    pub files: Vec<CommitFileStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileStat {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub branch: String,
    pub is_detached: bool,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub ahead: i64,
    pub behind: i64,
    pub remote_urls: HashMap<String, String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_repo: bool,
    pub has_repo_descendant: bool,
    pub repo_status: Option<RepoStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub old_line_no: i32,
    pub new_line_no: i32,
    pub content: String,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub hunks: Vec<HunkDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkDiff {
    pub old_start: u32,
    pub new_start: u32,
    pub old_lines: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

fn config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| format!("HOME env var not set: {}", e))?;
    Ok(PathBuf::from(home).join(".config/git-explorer/config.json"))
}

// C1: removed certificate_check bypass — libgit2 uses system cert validation.
fn make_remote_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
            .or_else(|_| git2::Cred::default())
    });
    cb
}

fn make_fetch_options() -> git2::FetchOptions<'static> {
    let cb = make_remote_callbacks();
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(cb);
    opts
}

// M5: branch name validation helper
fn is_valid_branch_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.starts_with('.') || name.starts_with('/') || name.starts_with('-') {
        return false;
    }
    if name.ends_with('.') || name.ends_with('/') {
        return false;
    }
    if name.contains("..") || name.contains("@{") {
        return false;
    }
    if name.contains(|c: char| {
        c.is_whitespace()
            || c == '~'
            || c == '^'
            || c == ':'
            || c == '?'
            || c == '*'
            || c == '['
            || c == '\\'
            || c == '\0'
    }) {
        return false;
    }
    true
}

fn compute_repo_status(repo: &git2::Repository, do_fetch: bool) -> RepoStatus {
    let head = repo.head();
    let (branch, is_detached) = match head {
        Ok(head_ref) => {
            if head_ref.is_branch() {
                let name = head_ref.shorthand().unwrap_or("(unknown)").to_string();
                (name, false)
            } else {
                ("(detached)".to_string(), true)
            }
        }
        Err(_) => ("(no HEAD)".to_string(), true),
    };

    let staged_count = count_staged(repo);
    let (unstaged_count, untracked_count) = count_unstaged(repo);
    let (ahead, behind, remote_urls) = compute_remote_info(repo, &branch, is_detached, do_fetch);

    // M1: precedence: dirty > diverged > ahead > behind > no-remote > clean
    let status = if staged_count > 0 || unstaged_count > 0 || untracked_count > 0 {
        "dirty".to_string()
    } else if ahead > 0 && behind > 0 {
        "diverged".to_string()
    } else if ahead > 0 {
        "ahead".to_string()
    } else if behind > 0 {
        "behind".to_string()
    } else if ahead == -1 && behind == -1 {
        "no-remote".to_string()
    } else {
        "clean".to_string()
    };

    RepoStatus {
        branch,
        is_detached,
        staged_count,
        unstaged_count,
        untracked_count,
        ahead,
        behind,
        remote_urls,
        status,
    }
}

fn count_staged(repo: &git2::Repository) -> usize {
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let index = match repo.index() {
        Ok(idx) => idx,
        Err(_) => return 0,
    };

    match head_tree {
        Some(tree) => {
            let diff = repo.diff_tree_to_index(Some(&tree), Some(&index), None);
            match diff {
                Ok(d) => d.deltas().len(),
                Err(_) => 0,
            }
        }
        None => {
            let diff = repo.diff_tree_to_index(None::<&git2::Tree>, Some(&index), None);
            match diff {
                Ok(d) => d.deltas().len(),
                Err(_) => 0,
            }
        }
    }
}

fn count_unstaged(repo: &git2::Repository) -> (usize, usize) {
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let diff = match repo.diff_index_to_workdir(None, Some(&mut opts)) {
        Ok(d) => d,
        Err(_) => return (0, 0),
    };

    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for delta in diff.deltas() {
        match delta.status() {
            git2::Delta::Untracked => untracked += 1,
            _ => unstaged += 1,
        }
    }

    (unstaged, untracked)
}

fn compute_remote_info(
    repo: &git2::Repository,
    branch: &str,
    is_detached: bool,
    do_fetch: bool,
) -> (i64, i64, HashMap<String, String>) {
    let mut remote_urls = HashMap::new();

    if let Ok(remotes) = repo.remotes() {
        for remote_name in &remotes {
            if let Some(name) = remote_name {
                if let Ok(remote) = repo.find_remote(name) {
                    if let Some(url) = remote.url() {
                        remote_urls.insert(name.to_string(), url.to_string());
                    }
                }
            }
        }
    }

    if is_detached {
        return (-1, -1, remote_urls);
    }

    if do_fetch {
        if let Ok(mut remote) = repo.find_remote("origin") {
            let mut fetch_opts = make_fetch_options();
            let _ = remote.fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(&mut fetch_opts),
                None,
            );
        }
    }

    let local_oid = match repo.head().ok().and_then(|h| h.target()) {
        Some(oid) => oid,
        None => return (-1, -1, remote_urls),
    };

    let remote_ref = format!("refs/remotes/origin/{}", branch);
    let remote_oid = match repo.find_reference(&remote_ref) {
        Ok(r) => r.target().unwrap_or(git2::Oid::zero()),
        Err(_) => return (-1, -1, remote_urls),
    };

    if remote_oid.is_zero() {
        return (-1, -1, remote_urls);
    }

    match repo.graph_ahead_behind(local_oid, remote_oid) {
        Ok((a, b)) => (a as i64, b as i64, remote_urls),
        Err(_) => (-1, -1, remote_urls),
    }
}

fn delta_status_str(delta: git2::Delta) -> String {
    match delta {
        git2::Delta::Added => "added".to_string(),
        git2::Delta::Deleted => "deleted".to_string(),
        git2::Delta::Modified => "modified".to_string(),
        git2::Delta::Renamed => "renamed".to_string(),
        git2::Delta::Copied => "copied".to_string(),
        git2::Delta::Untracked => "untracked".to_string(),
        git2::Delta::Unreadable => "unreadable".to_string(),
        _ => "other".to_string(),
    }
}

#[tauri::command]
pub fn scan_directory(root: String) -> Result<Vec<TreeNode>, String> {
    let root_path = Path::new(&root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {}", root));
    }
    if !root_path.is_dir() {
        return Err(format!("Not a directory: {}", root));
    }

    let mut nodes: Vec<TreeNode> = Vec::new();

    let root_name = root_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.clone());

    // M3: use .exists() to handle both .git dirs and .git files (worktrees/submodules)
    let root_is_repo = root_path.join(".git").exists();

    nodes.push(TreeNode {
        path: root.clone(),
        name: root_name,
        depth: 0,
        is_repo: root_is_repo,
        has_repo_descendant: false,
        repo_status: None,
    });

    let walker = WalkDir::new(&root)
        .follow_links(false)
        .max_depth(10)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            true
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        if entry.depth() == 0 {
            continue;
        }

        let path = entry.path();
        // M3: use .exists() to handle both .git dirs and .git files (worktrees/submodules)
        let is_repo = path.join(".git").exists();

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let depth = entry.depth();

        nodes.push(TreeNode {
            path: path.to_string_lossy().to_string(),
            name,
            depth,
            is_repo,
            has_repo_descendant: false,
            repo_status: None,
        });
    }

    let repo_paths: Vec<String> = nodes
        .iter()
        .filter(|n| n.is_repo)
        .map(|n| n.path.clone())
        .collect();

    for node in &mut nodes {
        if node.is_repo {
            node.has_repo_descendant = true;
            continue;
        }
        for repo_path in &repo_paths {
            // H2: require trailing slash to prevent /foo matching /foo-backup
            if repo_path.starts_with(&format!("{}/", node.path)) {
                node.has_repo_descendant = true;
                break;
            }
        }
    }

    for node in &mut nodes {
        if node.is_repo {
            match git2::Repository::open(&node.path) {
                Ok(repo) => {
                    node.repo_status = Some(compute_repo_status(&repo, false));
                }
                Err(_) => {
                    // M2: use "error" status instead of "dirty" for failed repo opens
                    node.repo_status = Some(RepoStatus {
                        branch: "(error)".to_string(),
                        is_detached: true,
                        staged_count: 0,
                        unstaged_count: 0,
                        untracked_count: 0,
                        ahead: -1,
                        behind: -1,
                        remote_urls: HashMap::new(),
                        status: "error".to_string(),
                    });
                }
            }
        }
    }

    Ok(nodes)
}

#[tauri::command]
pub fn get_repo_status(path: String) -> Result<RepoStatus, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("Failed to open repo at {}: {}", path, e))?;

    Ok(compute_repo_status(&repo, false))
}

#[tauri::command]
pub fn get_commit_log(path: String, limit: u32) -> Result<Vec<Commit>, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("Failed to open repo at {}: {}", path, e))?;

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;

    let head_commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel HEAD to commit: {}", e))?;

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    revwalk
        .push(head_commit.id())
        .map_err(|e| format!("Failed to push HEAD to revwalk: {}", e))?;

    let mut commits = Vec::new();

    for oid in revwalk.take(limit as usize) {
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hash = commit.id().to_string();
        let short_hash = hash[..7].to_string();
        let full_message = commit.message().unwrap_or("");
        let message = full_message.lines().next().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("(unknown)").to_string();
        let email = commit.author().email().unwrap_or("").to_string();
        let timestamp = commit.author().when().seconds();

        commits.push(Commit {
            hash,
            short_hash,
            message,
            author,
            email,
            timestamp,
        });
    }

    Ok(commits)
}

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

// C2: git_push (git2-native) removed — UI uses git_push_system exclusively.
// C2: git_pull (git2-native) removed — UI uses git_pull_system exclusively.

#[tauri::command]
pub fn list_branches(path: String) -> Result<Vec<BranchInfo>, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    let mut branches = Vec::new();

    for branch_result in repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| format!("Failed to list branches: {}", e))?
    {
        let (branch, _branch_type) =
            branch_result.map_err(|e| format!("Failed to read branch: {}", e))?;
        let name = branch
            .name()
            .map_err(|e| format!("Failed to get branch name: {}", e))?;
        let name_str = name.unwrap_or("(invalid)").to_string();
        let is_current = current_branch.as_ref() == Some(&name_str);

        branches.push(BranchInfo {
            name: name_str,
            is_current,
            is_remote: false,
        });
    }

    for branch_result in repo
        .branches(Some(git2::BranchType::Remote))
        .map_err(|e| format!("Failed to list remote branches: {}", e))?
    {
        let (branch, _branch_type) =
            branch_result.map_err(|e| format!("Failed to read branch: {}", e))?;
        let name = branch
            .name()
            .map_err(|e| format!("Failed to get branch name: {}", e))?;
        let name_str = name.unwrap_or("(invalid)").to_string();

        if name_str.ends_with("/HEAD") {
            continue;
        }

        branches.push(BranchInfo {
            name: name_str,
            is_current: false,
            is_remote: true,
        });
    }

    Ok(branches)
}

#[tauri::command]
pub fn checkout_branch(path: String, branch_name: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let statuses = repo
        .statuses(Some(git2::StatusOptions::new().include_untracked(true)))
        .map_err(|e| format!("Failed to get statuses: {}", e))?;

    let has_dirty = statuses.iter().any(|s| {
        let s = s.status();
        s.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED,
        )
    });

    if has_dirty {
        return Err(
            "Cannot switch branches: you have uncommitted changes. Commit or stash them first."
                .to_string(),
        );
    }

    // H3: detect remote branch names (contain '/' and refs/remotes/{name} exists)
    let is_remote_branch = branch_name.contains('/')
        && repo
            .find_reference(&format!("refs/remotes/{}", branch_name))
            .is_ok();

    if is_remote_branch {
        // Strip the remote prefix (e.g. "origin/feature" -> "feature")
        let slash_pos = branch_name.find('/').unwrap();
        let remote_name = &branch_name[..slash_pos];
        let local_name = &branch_name[slash_pos + 1..];

        // Get the remote commit to point local branch at
        let remote_ref = repo
            .find_reference(&format!("refs/remotes/{}", branch_name))
            .map_err(|e| format!("Failed to find remote ref: {}", e))?;
        let remote_oid = remote_ref
            .target()
            .ok_or("Remote ref has no target")?;
        let commit = repo
            .find_commit(remote_oid)
            .map_err(|e| format!("Failed to find remote commit: {}", e))?;

        // Create local branch if it doesn't exist; if it does, just check it out
        if repo.find_branch(local_name, git2::BranchType::Local).is_err() {
            let mut local_branch = repo
                .branch(local_name, &commit, false)
                .map_err(|e| format!("Failed to create local branch: {}", e))?;
            local_branch
                .set_upstream(Some(&format!("{}/{}", remote_name, local_name)))
                .map_err(|e| format!("Failed to set upstream: {}", e))?;
        }

        let ref_name = format!("refs/heads/{}", local_name);
        repo.set_head(&ref_name)
            .map_err(|e| format!("Failed to set HEAD to {}: {}", local_name, e))?;
        repo.checkout_head(None)
            .map_err(|e| format!("Failed to checkout {}: {}", local_name, e))?;
    } else {
        let ref_name = format!("refs/heads/{}", branch_name);
        repo.set_head(&ref_name)
            .map_err(|e| format!("Failed to set HEAD to {}: {}", branch_name, e))?;
        repo.checkout_head(None)
            .map_err(|e| format!("Failed to checkout {}: {}", branch_name, e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config {
            root_path: String::new(),
        });
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config)
}

#[tauri::command]
pub fn save_config(root_path: String) -> Result<(), String> {
    let path = config_path()?;

    let validated = Path::new(&root_path);
    if !validated.exists() || !validated.is_dir() {
        return Err(format!("Invalid directory: {}", root_path));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config = Config { root_path };
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

// H1: scope guard to ensure PID is cleared on both success and error paths
struct PidGuard<'a> {
    state: &'a AppState,
}

impl<'a> Drop for PidGuard<'a> {
    fn drop(&mut self) {
        let mut pid_lock = self.state.git_op_pid.lock().unwrap();
        *pid_lock = None;
    }
}

fn run_git_cancellable(repo_path: &str, args: &[&str], state: &AppState) -> Result<String, String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(repo_path);
    for arg in args {
        cmd.arg(arg);
    }

    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn git: {}", e))?;

    let pid = child.id();
    {
        let mut pid_lock = state.git_op_pid.lock().unwrap();
        *pid_lock = Some(pid);
    }

    // H1: guard clears PID on drop, covering both success and error paths
    let _guard = PidGuard { state };

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to wait for git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { "Git command failed".to_string() } else { stderr.trim().to_string() })
    }
}

#[tauri::command]
pub fn git_fetch(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    run_git_cancellable(&path, &["fetch", "--all"], &state)?;
    Ok(())
}

#[tauri::command]
pub fn git_push_system(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let branch = {
        let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
        let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
        head.shorthand().ok_or("No branch name")?.to_string()
    };

    match run_git_cancellable(&path, &["push", "origin", &branch], &state) {
        Ok(_) => Ok(()),
        Err(e) if e.contains("no upstream branch") => {
            let args: Vec<&str> = vec!["push", "--set-upstream", "origin", &branch];
            run_git_cancellable(&path, &args, &state)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn git_pull_system(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    run_git_cancellable(&path, &["pull"], &state)?;
    Ok(())
}

#[tauri::command]
pub fn cancel_git_op(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // M4: read PID under lock, drop guard, then kill — don't hold mutex across syscall
    let pid = {
        let pid_lock = state.git_op_pid.lock().unwrap();
        *pid_lock
    };
    if let Some(pid) = pid {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        Ok(())
    } else {
        Err("No git operation in progress".to_string())
    }
}

#[tauri::command]
pub fn discard_file(path: String, file_path: String) -> Result<(), String> {
    let result = std::process::Command::new("git")
        .arg("-C").arg(&path)
        .arg("checkout").arg("HEAD").arg("--").arg(&file_path)
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(e) => Err(format!("Failed to run git: {}", e)),
    }
}

#[tauri::command]
pub fn create_branch(path: String, branch_name: String, checkout: bool) -> Result<(), String> {
    // M5: use expanded validation helper
    if !is_valid_branch_name(&branch_name) {
        return Err("Invalid branch name".to_string());
    }

    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_oid = head.target().ok_or("No HEAD target")?;
    let head_commit = repo.find_commit(head_oid).map_err(|e| format!("Failed to find HEAD commit: {}", e))?;

    repo.branch(&branch_name, &head_commit, false)
        .map_err(|e| format!("Failed to create branch: {}", e))?;

    if checkout {
        let ref_name = format!("refs/heads/{}", branch_name);
        repo.set_head(&ref_name)
            .map_err(|e| format!("Failed to set HEAD: {}", e))?;
        repo.checkout_head(None)
            .map_err(|e| format!("Failed to checkout: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn delete_branch(path: String, branch_name: String) -> Result<(), String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let mut branch = repo.find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|e| format!("Failed to find branch: {}", e))?;
    branch.delete().map_err(|e| format!("Failed to delete branch: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn git_merge_branch(path: String, branch_name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // M5: use expanded validation helper
    if !is_valid_branch_name(&branch_name) {
        return Err("Invalid branch name".to_string());
    }
    run_git_cancellable(&path, &["merge", &branch_name], &state)?;
    Ok(())
}

#[tauri::command]
pub fn git_stash_save(path: String) -> Result<(), String> {
    let mut repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    // L2: error on missing identity instead of using a fallback
    let sig = repo.signature().map_err(|_| {
        "No git identity configured. Run: git config --global user.name \"Your Name\" && git config --global user.email \"you@example.com\"".to_string()
    })?;

    repo.stash_save(&sig, "Git Explorer stash", None)
        .map_err(|e| format!("Failed to stash: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn git_stash_pop(path: String) -> Result<(), String> {
    let mut repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_pop(0, None)
        .map_err(|e| format!("Failed to pop stash: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn git_stash_apply(path: String, index: usize) -> Result<(), String> {
    let mut repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_apply(index, None)
        .map_err(|e| format!("Failed to apply stash: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn git_stash_drop(path: String, index: usize) -> Result<(), String> {
    let mut repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_drop(index)
        .map_err(|e| format!("Failed to drop stash: {}", e))?;
    Ok(())
}

// L1: use stash_foreach instead of reflog iteration for canonical stash indices
#[tauri::command]
pub fn git_stash_list(path: String) -> Result<Vec<StashEntry>, String> {
    let mut repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let mut entries = Vec::new();
    repo.stash_foreach(|index, message, _oid| {
        entries.push(StashEntry { index, message: message.to_string() });
        true
    }).map_err(|e| format!("Failed to list stashes: {}", e))?;
    Ok(entries)
}

#[tauri::command]
pub fn get_commit_details(path: String, oid: String) -> Result<CommitDetail, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let commit_oid = git2::Oid::from_str(&oid).map_err(|e| format!("Invalid OID: {}", e))?;
    let commit = repo.find_commit(commit_oid).map_err(|e| format!("Failed to find commit: {}", e))?;

    let message = commit.message().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("(unknown)").to_string();
    let timestamp = commit.author().when().seconds();

    let tree = commit.tree().map_err(|e| format!("Failed to get tree: {}", e))?;
    let parent = commit.parent(0).ok();
    let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());

    let diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&tree),
        None,
    ).map_err(|e| format!("Failed to get diff: {}", e))?;

    let mut file_stats: Vec<CommitFileStat> = Vec::new();
    for delta in diff.deltas() {
        let file_path = delta.new_file().path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let status = delta_status_str(delta.status());
        file_stats.push(CommitFileStat {
            path: file_path,
            additions: 0,
            deletions: 0,
            status,
        });
    }

    let mut file_additions = vec![0usize; file_stats.len()];
    let mut file_deletions = vec![0usize; file_stats.len()];

    let diff2 = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&tree),
        None,
    ).map_err(|e| format!("Failed to get diff: {}", e))?;

    // H4: track file index by enumeration in the delta callback, avoiding the
    // None-path collision bug where all deleted files collapsed to index 0.
    let current_file_idx = std::cell::RefCell::new(0usize);
    let delta_counter = std::cell::RefCell::new(0usize);

    diff2.foreach(
        &mut |_delta, _progress| {
            let idx = *delta_counter.borrow();
            *current_file_idx.borrow_mut() = idx;
            *delta_counter.borrow_mut() = idx + 1;
            true
        },
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
            let idx = *current_file_idx.borrow();
            if idx < file_additions.len() {
                match line.origin() {
                    '+' => file_additions[idx] += 1,
                    '-' => file_deletions[idx] += 1,
                    _ => {}
                }
            }
            true
        }),
    ).unwrap_or(());

    for (i, stat) in file_stats.iter_mut().enumerate() {
        stat.additions = file_additions[i];
        stat.deletions = file_deletions[i];
    }

    Ok(CommitDetail {
        oid: oid.clone(),
        message,
        author,
        timestamp,
        files: file_stats,
    })
}
