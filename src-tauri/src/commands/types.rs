use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
