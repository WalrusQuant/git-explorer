use super::helpers::{is_valid_branch_name, run_git_cancellable};
use super::AppState;

#[tauri::command]
pub fn git_fetch(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    run_git_cancellable(&path, &["fetch", "--all"], &state)?;
    Ok(())
}

#[tauri::command]
pub fn git_push_system(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let branch = {
        let repo =
            git2::Repository::open(&path).map_err(|e| format!("Failed to open repo: {}", e))?;
        let head = repo
            .head()
            .map_err(|e| format!("Failed to get HEAD: {}", e))?;
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
pub fn git_merge_branch(
    path: String,
    branch_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // M5: use expanded validation helper
    if !is_valid_branch_name(&branch_name) {
        return Err("Invalid branch name".to_string());
    }
    run_git_cancellable(&path, &["merge", &branch_name], &state)?;
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
