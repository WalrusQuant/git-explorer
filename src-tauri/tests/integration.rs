use git_explorer_lib::commands;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn init_repo(path: &Path) {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("main");
    git2::Repository::init_opts(path, &opts).expect("init repo");

    let cfg_path = path.join(".git").join("config");
    let mut cfg = git2::Config::open(&cfg_path).expect("open repo config");
    cfg.set_str("user.name", "Test User").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();
}

fn make_repo() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();
    init_repo(&path);
    (dir, path)
}

fn write_file(repo: &Path, name: &str, content: &str) {
    let full = repo.join(name);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full, content).unwrap();
}

fn commit_all(repo: &Path, message: &str) -> String {
    let p = repo.to_string_lossy().to_string();
    commands::stage_all(p.clone()).expect("stage_all");
    commands::git_commit(p, message.to_string()).expect("commit")
}

// --- scan_directory ---

#[test]
fn scan_empty_dir_returns_only_root() {
    let dir = tempfile::tempdir().unwrap();
    let nodes = commands::scan_directory(dir.path().to_string_lossy().to_string()).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].depth, 0);
    assert!(!nodes[0].is_repo);
    assert!(!nodes[0].has_repo_descendant);
}

#[test]
fn scan_finds_repo_subdir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("plain")).unwrap();
    let repo_dir = root.join("with_git");
    fs::create_dir(&repo_dir).unwrap();
    init_repo(&repo_dir);

    let nodes = commands::scan_directory(root.to_string_lossy().to_string()).unwrap();
    let root_node = nodes.iter().find(|n| n.depth == 0).unwrap();
    assert!(root_node.has_repo_descendant, "root should mark descendant repo");
    assert!(!root_node.is_repo);

    let with_git = nodes.iter().find(|n| n.name == "with_git").unwrap();
    assert!(with_git.is_repo);
    assert!(with_git.has_repo_descendant);
    assert!(with_git.repo_status.is_some());

    let plain = nodes.iter().find(|n| n.name == "plain").unwrap();
    assert!(!plain.is_repo);
    assert!(!plain.has_repo_descendant);
}

#[test]
fn scan_marks_intermediate_ancestors() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let nested = root.join("a").join("b").join("c");
    fs::create_dir_all(&nested).unwrap();
    init_repo(&nested);

    let nodes = commands::scan_directory(root.to_string_lossy().to_string()).unwrap();
    for name in ["a", "b", "c"] {
        let n = nodes.iter().find(|n| n.name == name).expect(name);
        assert!(n.has_repo_descendant, "{} should have repo descendant", name);
    }
    let c = nodes.iter().find(|n| n.name == "c").unwrap();
    assert!(c.is_repo);
}

#[test]
fn scan_skips_dotfile_directories() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let hidden = root.join(".hidden");
    fs::create_dir(&hidden).unwrap();
    init_repo(&hidden);

    let nodes = commands::scan_directory(root.to_string_lossy().to_string()).unwrap();
    assert!(nodes.iter().all(|n| n.name != ".hidden"));
    let root_node = nodes.iter().find(|n| n.depth == 0).unwrap();
    assert!(!root_node.has_repo_descendant);
}

// Regression for the H2 fix: matching by string prefix without a trailing slash
// caused `/foo` to claim `/foo-backup` as a descendant.
#[test]
fn scan_prefix_does_not_match_sibling_with_same_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let foo = root.join("foo");
    fs::create_dir(&foo).unwrap();
    init_repo(&foo);
    fs::create_dir(root.join("foo-backup")).unwrap();

    let nodes = commands::scan_directory(root.to_string_lossy().to_string()).unwrap();
    let backup = nodes.iter().find(|n| n.name == "foo-backup").unwrap();
    assert!(!backup.has_repo_descendant, "sibling with prefix overlap must not be flagged");
}

#[test]
fn scan_errors_on_missing_dir() {
    let result = commands::scan_directory("/does/not/exist/git-explorer-test".to_string());
    assert!(result.is_err());
}

// --- repo status ---

#[test]
fn status_no_head_for_fresh_repo() {
    let (_d, path) = make_repo();
    let s = commands::get_repo_status(path.to_string_lossy().to_string()).unwrap();
    assert!(s.is_detached);
    assert_eq!(s.ahead, -1);
    assert_eq!(s.behind, -1);
}

#[test]
fn status_clean_after_commit() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");

    let s = commands::get_repo_status(path.to_string_lossy().to_string()).unwrap();
    assert_eq!(s.branch, "main");
    assert!(!s.is_detached);
    assert_eq!(s.staged_count, 0);
    assert_eq!(s.unstaged_count, 0);
    assert_eq!(s.untracked_count, 0);
    // no remote configured for a temp repo
    assert_eq!(s.status, "no-remote");
}

#[test]
fn status_dirty_with_untracked_file() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");
    write_file(&path, "new.txt", "new\n");

    let s = commands::get_repo_status(path.to_string_lossy().to_string()).unwrap();
    assert_eq!(s.untracked_count, 1);
    assert_eq!(s.status, "dirty");
}

#[test]
fn status_dirty_with_modified_tracked_file() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");
    write_file(&path, "README.md", "changed\n");

    let s = commands::get_repo_status(path.to_string_lossy().to_string()).unwrap();
    assert_eq!(s.unstaged_count, 1);
    assert_eq!(s.status, "dirty");
}

// --- staging + commit ---

#[test]
fn stage_then_unstage_roundtrip() {
    let (_d, path) = make_repo();
    write_file(&path, "a.txt", "one\n");
    commit_all(&path, "init");

    write_file(&path, "a.txt", "two\n");
    let p = path.to_string_lossy().to_string();

    commands::stage_file(p.clone(), "a.txt".to_string()).unwrap();
    let changed = commands::get_changed_files(p.clone()).unwrap();
    let staged: Vec<_> = changed.iter().filter(|f| f.staged).collect();
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].path, "a.txt");

    commands::unstage_file(p.clone(), "a.txt".to_string()).unwrap();
    let changed = commands::get_changed_files(p.clone()).unwrap();
    assert!(changed.iter().all(|f| !f.staged));
    let unstaged: Vec<_> = changed.iter().filter(|f| !f.staged && f.path == "a.txt").collect();
    assert_eq!(unstaged.len(), 1);
}

#[test]
fn commit_creates_entry_in_log() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    let oid = commit_all(&path, "init commit");

    let log = commands::get_commit_log(path.to_string_lossy().to_string(), 10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].hash, oid);
    assert_eq!(log[0].short_hash.len(), 7);
    assert_eq!(log[0].message, "init commit");
    assert_eq!(log[0].author, "Test User");
    assert_eq!(log[0].email, "test@example.com");
}

#[test]
fn commit_log_respects_limit() {
    let (_d, path) = make_repo();
    write_file(&path, "a.txt", "1\n");
    commit_all(&path, "one");
    write_file(&path, "a.txt", "2\n");
    commit_all(&path, "two");
    write_file(&path, "a.txt", "3\n");
    commit_all(&path, "three");

    let log = commands::get_commit_log(path.to_string_lossy().to_string(), 2).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].message, "three");
    assert_eq!(log[1].message, "two");
}

// --- diff ---

#[test]
fn file_diff_reports_added_line() {
    let (_d, path) = make_repo();
    write_file(&path, "a.txt", "hello\n");
    commit_all(&path, "init");
    write_file(&path, "a.txt", "hello\nworld\n");

    let diff = commands::get_file_diff(
        path.to_string_lossy().to_string(),
        "a.txt".to_string(),
        false,
    )
    .unwrap();
    assert!(!diff.hunks.is_empty());
    let added: Vec<_> = diff
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .filter(|l| l.origin == "add")
        .collect();
    assert!(added.iter().any(|l| l.content.contains("world")));
}

// --- branches ---

#[test]
fn branches_create_list_delete() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");
    let p = path.to_string_lossy().to_string();

    commands::create_branch(p.clone(), "feature".to_string(), false).unwrap();
    let branches = commands::list_branches(p.clone()).unwrap();
    let names: Vec<_> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"feature"));

    let main = branches.iter().find(|b| b.name == "main").unwrap();
    assert!(main.is_current);
    let feature = branches.iter().find(|b| b.name == "feature").unwrap();
    assert!(!feature.is_current);

    commands::delete_branch(p.clone(), "feature".to_string()).unwrap();
    let branches = commands::list_branches(p).unwrap();
    assert!(branches.iter().all(|b| b.name != "feature"));
}

#[test]
fn checkout_branch_switches_head() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");
    let p = path.to_string_lossy().to_string();

    commands::create_branch(p.clone(), "feature".to_string(), true).unwrap();
    let s = commands::get_repo_status(p).unwrap();
    assert_eq!(s.branch, "feature");
}

#[test]
fn create_branch_rejects_invalid_name() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");

    let err = commands::create_branch(
        path.to_string_lossy().to_string(),
        "bad name".to_string(),
        false,
    )
    .unwrap_err();
    assert!(err.contains("Invalid"));
}

#[test]
fn checkout_blocked_when_dirty() {
    let (_d, path) = make_repo();
    write_file(&path, "README.md", "hi\n");
    commit_all(&path, "init");
    let p = path.to_string_lossy().to_string();

    commands::create_branch(p.clone(), "feature".to_string(), false).unwrap();
    write_file(&path, "README.md", "uncommitted\n");

    let err = commands::checkout_branch(p, "feature".to_string()).unwrap_err();
    assert!(err.contains("uncommitted"));
}
