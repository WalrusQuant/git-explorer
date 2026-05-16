use super::AppState;
use std::path::PathBuf;

pub(crate) fn config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| format!("HOME env var not set: {}", e))?;
    Ok(PathBuf::from(home).join(".config/git-explorer/config.json"))
}

// C1: removed certificate_check bypass — libgit2 uses system cert validation.
pub(crate) fn make_remote_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
            .or_else(|_| git2::Cred::default())
    });
    cb
}

pub(crate) fn make_fetch_options() -> git2::FetchOptions<'static> {
    let cb = make_remote_callbacks();
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(cb);
    opts
}

// M5: branch name validation helper
pub(crate) fn is_valid_branch_name(name: &str) -> bool {
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

pub(crate) fn delta_status_str(delta: git2::Delta) -> String {
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

// H1: scope guard to ensure PID is cleared on both success and error paths
pub(crate) struct PidGuard<'a> {
    pub(crate) state: &'a AppState,
}

impl<'a> Drop for PidGuard<'a> {
    fn drop(&mut self) {
        let mut pid_lock = self.state.git_op_pid.lock().unwrap();
        *pid_lock = None;
    }
}

pub(crate) fn run_git_cancellable(
    repo_path: &str,
    args: &[&str],
    state: &AppState,
) -> Result<String, String> {
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

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() {
            "Git command failed".to_string()
        } else {
            stderr.trim().to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_name_accepts_valid_names() {
        for name in [
            "main",
            "master",
            "feature/foo",
            "feature/foo/bar",
            "v1.0",
            "release-2026",
            "user/adam/wip",
            "fix_typo",
        ] {
            assert!(is_valid_branch_name(name), "expected valid: {}", name);
        }
    }

    #[test]
    fn branch_name_rejects_invalid_names() {
        let bad = [
            "",
            ".hidden",
            "/leading",
            "-dash",
            "trailing.",
            "trailing/",
            "double..dot",
            "ref@{0}",
            "has space",
            "has~tilde",
            "has^caret",
            "has:colon",
            "has?q",
            "has*star",
            "has[bracket",
            "has\\back",
            "has\0null",
        ];
        for name in bad {
            assert!(!is_valid_branch_name(name), "expected invalid: {:?}", name);
        }
    }

    #[test]
    fn delta_status_maps_each_variant() {
        assert_eq!(delta_status_str(git2::Delta::Added), "added");
        assert_eq!(delta_status_str(git2::Delta::Deleted), "deleted");
        assert_eq!(delta_status_str(git2::Delta::Modified), "modified");
        assert_eq!(delta_status_str(git2::Delta::Renamed), "renamed");
        assert_eq!(delta_status_str(git2::Delta::Copied), "copied");
        assert_eq!(delta_status_str(git2::Delta::Untracked), "untracked");
        assert_eq!(delta_status_str(git2::Delta::Unreadable), "unreadable");
        assert_eq!(delta_status_str(git2::Delta::Ignored), "other");
        assert_eq!(delta_status_str(git2::Delta::Unmodified), "other");
    }
}
