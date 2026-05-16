use super::helpers::delta_status_str;
use super::types::{Commit, CommitDetail, CommitFileStat};

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
pub fn get_commit_details(path: String, oid: String) -> Result<CommitDetail, String> {
    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let commit_oid = git2::Oid::from_str(&oid).map_err(|e| format!("Invalid OID: {}", e))?;
    let commit = repo
        .find_commit(commit_oid)
        .map_err(|e| format!("Failed to find commit: {}", e))?;

    let message = commit.message().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("(unknown)").to_string();
    let timestamp = commit.author().when().seconds();

    let tree = commit
        .tree()
        .map_err(|e| format!("Failed to get tree: {}", e))?;
    let parent = commit.parent(0).ok();
    let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|e| format!("Failed to get diff: {}", e))?;

    let mut file_stats: Vec<CommitFileStat> = Vec::new();
    for delta in diff.deltas() {
        let file_path = delta
            .new_file()
            .path()
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

    let diff2 = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|e| format!("Failed to get diff: {}", e))?;

    // H4: track file index by enumeration in the delta callback, avoiding the
    // None-path collision bug where all deleted files collapsed to index 0.
    let current_file_idx = std::cell::RefCell::new(0usize);
    let delta_counter = std::cell::RefCell::new(0usize);

    diff2
        .foreach(
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
        )
        .unwrap_or(());

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
