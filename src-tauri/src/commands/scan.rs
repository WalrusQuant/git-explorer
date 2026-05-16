use super::status::compute_repo_status;
use super::types::{RepoStatus, TreeNode};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

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
