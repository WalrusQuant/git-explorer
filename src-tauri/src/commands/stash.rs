use super::types::StashEntry;

#[tauri::command]
pub fn git_stash_save(path: String) -> Result<(), String> {
    let mut repo =
        git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
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
    let mut repo =
        git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_pop(0, None)
        .map_err(|e| format!("Failed to pop stash: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn git_stash_apply(path: String, index: usize) -> Result<(), String> {
    let mut repo =
        git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_apply(index, None)
        .map_err(|e| format!("Failed to apply stash: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn git_stash_drop(path: String, index: usize) -> Result<(), String> {
    let mut repo =
        git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    repo.stash_drop(index)
        .map_err(|e| format!("Failed to drop stash: {}", e))?;
    Ok(())
}

// L1: use stash_foreach instead of reflog iteration for canonical stash indices
#[tauri::command]
pub fn git_stash_list(path: String) -> Result<Vec<StashEntry>, String> {
    let mut repo =
        git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
    let mut entries = Vec::new();
    repo.stash_foreach(|index, message, _oid| {
        entries.push(StashEntry {
            index,
            message: message.to_string(),
        });
        true
    })
    .map_err(|e| format!("Failed to list stashes: {}", e))?;
    Ok(entries)
}
