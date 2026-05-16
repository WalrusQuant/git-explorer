use super::helpers::make_fetch_options;
use super::types::RepoStatus;
use std::collections::HashMap;

pub(crate) fn compute_repo_status(repo: &git2::Repository, do_fetch: bool) -> RepoStatus {
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
            if let Err(err) = remote.fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(&mut fetch_opts),
                None,
            ) {
                tracing::warn!(
                    repo = ?repo.path(),
                    "background fetch from origin failed: {}",
                    err
                );
            }
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

#[tauri::command]
pub fn get_repo_status(path: String) -> Result<RepoStatus, String> {
    let repo = git2::Repository::open(&path)
        .map_err(|e| format!("Failed to open repo at {}: {}", path, e))?;

    Ok(compute_repo_status(&repo, false))
}
