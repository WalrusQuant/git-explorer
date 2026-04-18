mod commands;

use commands::AppState;
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        git_op_pid: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::scan_directory,
            commands::get_repo_status,
            commands::get_commit_log,
            commands::load_config,
            commands::save_config,
            commands::get_changed_files,
            commands::get_file_diff,
            commands::stage_file,
            commands::unstage_file,
            commands::stage_all,
            commands::unstage_all,
            commands::git_commit,
            commands::list_branches,
            commands::checkout_branch,
            commands::git_fetch,
            commands::git_push_system,
            commands::git_pull_system,
            commands::cancel_git_op,
            commands::discard_file,
            commands::create_branch,
            commands::delete_branch,
            commands::git_merge_branch,
            commands::git_stash_save,
            commands::git_stash_pop,
            commands::git_stash_apply,
            commands::git_stash_drop,
            commands::git_stash_list,
            commands::get_commit_details,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
