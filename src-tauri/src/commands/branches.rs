use super::helpers::is_valid_branch_name;
use super::types::BranchInfo;

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
        let remote_oid = remote_ref.target().ok_or("Remote ref has no target")?;
        let commit = repo
            .find_commit(remote_oid)
            .map_err(|e| format!("Failed to find remote commit: {}", e))?;

        // Create local branch if it doesn't exist; if it does, just check it out
        if repo
            .find_branch(local_name, git2::BranchType::Local)
            .is_err()
        {
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
pub fn create_branch(path: String, branch_name: String, checkout: bool) -> Result<(), String> {
    // M5: use expanded validation helper
    if !is_valid_branch_name(&branch_name) {
        return Err("Invalid branch name".to_string());
    }

    let repo = git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_oid = head.target().ok_or("No HEAD target")?;
    let head_commit = repo
        .find_commit(head_oid)
        .map_err(|e| format!("Failed to find HEAD commit: {}", e))?;

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
    let mut branch = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|e| format!("Failed to find branch: {}", e))?;
    branch
        .delete()
        .map_err(|e| format!("Failed to delete branch: {}", e))?;
    Ok(())
}
