export type RepoStatusIndicator = 'clean' | 'dirty' | 'ahead' | 'behind' | 'diverged' | 'no-remote' | 'error';

export interface RepoStatus {
  branch: string;
  is_detached: boolean;
  staged_count: number;
  unstaged_count: number;
  untracked_count: number;
  ahead: number;
  behind: number;
  remote_urls: Record<string, string>;
  status: RepoStatusIndicator;
}

export interface TreeNode {
  path: string;
  name: string;
  depth: number;
  is_repo: boolean;
  has_repo_descendant: boolean;
  repo_status: RepoStatus | null;
}

export interface Commit {
  hash: string;
  short_hash: string;
  message: string;
  author: string;
  email: string;
  timestamp: number;
}

export interface CommitDetail {
  oid: string;
  message: string;
  author: string;
  timestamp: number;
  files: CommitFileStat[];
}

export interface CommitFileStat {
  path: string;
  additions: number;
  deletions: number;
  status: string;
}

export interface Config {
  root_path: string;
}

export interface FileChange {
  path: string;
  old_path: string | null;
  status: 'added' | 'deleted' | 'modified' | 'renamed' | 'copied' | 'untracked' | 'unreadable' | 'other';
  staged: boolean;
}

export interface DiffLine {
  old_line_no: number;
  new_line_no: number;
  content: string;
  origin: 'add' | 'remove' | 'context';
}

export interface HunkDiff {
  old_start: number;
  new_start: number;
  old_lines: number;
  new_lines: number;
  lines: DiffLine[];
}

export interface FileDiff {
  old_path: string | null;
  new_path: string | null;
  hunks: HunkDiff[];
}

export interface BranchInfo {
  name: string;
  is_current: boolean;
  is_remote: boolean;
}

export interface StashEntry {
  index: number;
  message: string;
}

export interface ToastMessage {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info';
}
