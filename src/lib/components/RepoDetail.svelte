<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { TreeNode, Commit, CommitDetail, BranchInfo, StashEntry, ToastMessage } from '../types';
  import ChangedFiles from './ChangedFiles.svelte';
  import CommitDialog from './CommitDialog.svelte';

  let {
    repo,
    commits,
    onRefresh,
    onToast,
  }: {
    repo: TreeNode;
    commits: Commit[];
    onRefresh: () => void;
    onToast: (msg: string, type: ToastMessage['type']) => void;
  } = $props();

  let activeTab: 'overview' | 'changes' | 'commits' = $state('overview');
  let showCommit: boolean = $state(false);
  let branches: BranchInfo[] = $state([]);
  let stashes: StashEntry[] = $state([]);
  let showBranchMenu: boolean = $state(false);
  let newBranchName: string = $state('');
  let showNewBranch: boolean = $state(false);
  let expandedCommit: string | null = $state(null);
  let commitDetail: CommitDetail | null = $state(null);
  let pushing: boolean = $state(false);
  let pulling: boolean = $state(false);
  let fetching: boolean = $state(false);
  let stashing: boolean = $state(false);
  let popping: boolean = $state(false);
  let showStashSection: boolean = $state(false);
  let confirmDeleteBranch: string | null = $state(null);
  let confirmMergeBranch: string | null = $state(null);

  let status = $derived(repo.repo_status);

  let statusLabel = $derived.by(() => {
    if (!status) return '';
    switch (status.status) {
      case 'clean': return 'Clean';
      case 'dirty': return 'Dirty';
      case 'ahead': return 'Ahead';
      case 'behind': return 'Behind';
      case 'diverged': return 'Diverged';
      case 'no-remote': return 'No remote';
      case 'error': return 'Error';
      default: return 'Unknown';
    }
  });

  let statusBg = $derived.by(() => {
    if (!status) return 'bg-gray-700 text-gray-300';
    switch (status.status) {
      case 'clean': return 'bg-badge-clean-bg text-badge-clean-text';
      case 'dirty': return 'bg-badge-dirty-bg text-badge-dirty-text';
      case 'ahead': return 'bg-badge-ahead-bg text-badge-ahead-text';
      case 'behind': return 'bg-badge-behind-bg text-badge-behind-text';
      case 'diverged': return 'bg-badge-diverged-bg text-badge-diverged-text';
      case 'no-remote': return 'bg-gray-700 text-gray-300';
      case 'error': return 'bg-red-900/40 text-red-300';
      default: return 'bg-gray-700 text-gray-300';
    }
  });

  async function loadBranches() {
    try {
      branches = await invoke<BranchInfo[]>('list_branches', { path: repo.path });
    } catch {
      branches = [];
    }
  }

  async function loadStashes() {
    try {
      stashes = await invoke<StashEntry[]>('git_stash_list', { path: repo.path });
    } catch {
      stashes = [];
    }
  }

  async function handleCheckout(branchName: string) {
    showBranchMenu = false;
    try {
      await invoke('checkout_branch', { path: repo.path, branchName });
      onToast(`Switched to ${branchName}`, 'success');
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleCreateBranch() {
    if (!newBranchName.trim()) return;
    try {
      await invoke('create_branch', { path: repo.path, branchName: newBranchName.trim(), checkout: true });
      onToast(`Created and switched to ${newBranchName.trim()}`, 'success');
      newBranchName = '';
      showNewBranch = false;
      onRefresh();
      loadBranches();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleDeleteBranch(branchName: string) {
    confirmDeleteBranch = null;
    try {
      await invoke('delete_branch', { path: repo.path, branchName });
      onToast(`Deleted branch ${branchName}`, 'success');
      loadBranches();
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleMergeBranch(branchName: string) {
    confirmMergeBranch = null;
    try {
      await invoke('git_merge_branch', { path: repo.path, branchName });
      onToast(`Merged ${branchName}`, 'success');
      onRefresh();
      loadBranches();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handlePush() {
    pushing = true;
    try {
      await invoke('git_push_system', { path: repo.path });
      onToast('Pushed successfully', 'success');
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
    pushing = false;
  }

  async function handlePull() {
    pulling = true;
    try {
      await invoke('git_pull_system', { path: repo.path });
      onToast('Pulled successfully', 'success');
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
    pulling = false;
  }

  async function handleFetch() {
    fetching = true;
    try {
      await invoke('git_fetch', { path: repo.path });
      onToast('Fetched from remote', 'success');
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
    fetching = false;
  }

  async function handleStashSave() {
    stashing = true;
    try {
      await invoke('git_stash_save', { path: repo.path });
      onToast('Changes stashed', 'success');
      onRefresh();
      loadStashes();
    } catch (e) {
      onToast(String(e), 'error');
    }
    stashing = false;
  }

  async function handleStashPop() {
    popping = true;
    try {
      await invoke('git_stash_pop', { path: repo.path });
      onToast('Stash popped', 'success');
      onRefresh();
      loadStashes();
    } catch (e) {
      onToast(String(e), 'error');
    }
    popping = false;
  }

  async function handleStashApply(index: number) {
    try {
      await invoke('git_stash_apply', { path: repo.path, index });
      onToast('Stash applied', 'success');
      onRefresh();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleStashDrop(index: number) {
    try {
      await invoke('git_stash_drop', { path: repo.path, index });
      onToast('Stash dropped', 'success');
      loadStashes();
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleExpandCommit(commitHash: string) {
    if (expandedCommit === commitHash) {
      expandedCommit = null;
      commitDetail = null;
      return;
    }
    expandedCommit = commitHash;
    commitDetail = null;
    try {
      commitDetail = await invoke<CommitDetail>('get_commit_details', { path: repo.path, oid: commitHash });
    } catch {
      commitDetail = null;
    }
  }

  function handleCommitDone() {
    showCommit = false;
    onRefresh();
  }

  function formatRelativeTime(timestamp: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - timestamp;
    if (diff < 60) return '< 1m ago';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 2592000) return `${Math.floor(diff / 86400)}d ago`;
    if (diff < 31536000) return `${Math.floor(diff / 2592000)}mo ago`;
    return `${Math.floor(diff / 31536000)}y ago`;
  }

  $effect(() => {
    if (showBranchMenu) {
      loadBranches();
    }
  });

  $effect(() => {
    if (showStashSection) {
      loadStashes();
    }
  });
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b border-gray-800 px-4 py-2.5">
    <div class="flex items-center gap-3">
      <h1 class="text-base font-semibold">{repo.name}</h1>

      {#if status}
        <div class="relative">
          <button
            onclick={() => (showBranchMenu = !showBranchMenu)}
            class="flex items-center gap-1.5 rounded-md bg-gray-800 px-2.5 py-1 font-mono text-xs text-gray-300 transition hover:bg-gray-700"
          >
            <svg class="h-3 w-3 text-gray-500" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11.75 2.5a.75.75 0 000 1.5c1.66 0 3 1.13 3 2.5s-1.34 2.5-3 2.5a.75.75 0 000 1.5c1.66 0 3 1.13 3 2.5s-1.34 2.5-3 2.5a.75.75 0 000 1.5c2.62 0 4.5-1.77 4.5-4 0-1.68-1.05-3.1-2.6-3.75A4.04 4.04 0 0016.25 6.5c0-2.23-1.88-4-4.5-4z" />
            </svg>
            {status.branch}
            <svg class="h-3 w-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </button>

          {#if showBranchMenu}
            <div class="absolute left-0 top-full z-50 mt-1 w-64 rounded-lg border border-gray-700 bg-gray-900 shadow-xl">
              <div class="max-h-56 overflow-y-auto p-1">
                {#each branches.filter((b) => !b.is_remote) as branch}
                  <div class="group/branch flex items-center gap-1 rounded-md px-2 py-1.5 text-sm transition hover:bg-gray-800 {branch.is_current ? 'text-blue-400' : 'text-gray-300'}">
                    {#if branch.is_current}
                      <span class="text-xs">●</span>
                    {:else}
                      <span class="text-xs text-gray-700">●</span>
                    {/if}
                    <button
                      class="flex-1 text-left font-mono text-xs"
                      onclick={() => handleCheckout(branch.name)}
                      disabled={branch.is_current}
                    >
                      {branch.name}
                    </button>
                    {#if !branch.is_current}
                      <button
                        class="text-xs text-gray-700 opacity-0 transition hover:text-green-400 group-hover/branch:opacity-100"
                        onclick={() => { confirmMergeBranch = branch.name; }}
                        title="Merge into current"
                      >⊕</button>
                      <button
                        class="text-xs text-gray-700 opacity-0 transition hover:text-red-400 group-hover/branch:opacity-100"
                        onclick={() => { confirmDeleteBranch = branch.name; }}
                        title="Delete branch"
                      >×</button>
                    {/if}
                  </div>
                {/each}
              </div>

              {#if branches.filter((b) => b.is_remote).length > 0}
                <div class="border-t border-gray-700/50 px-3 py-1 text-xs text-gray-600">Remote</div>
                <div class="max-h-32 overflow-y-auto p-1">
                  {#each branches.filter((b) => b.is_remote) as branch}
                    <div class="px-2 py-1 font-mono text-xs text-gray-500">{branch.name}</div>
                  {/each}
                </div>
              {/if}

              <div class="border-t border-gray-700/50 p-2">
                {#if showNewBranch}
                  <div class="flex gap-1">
                    <input
                      type="text"
                      bind:value={newBranchName}
                      placeholder="Branch name"
                      class="w-full rounded border border-gray-700 bg-gray-800 px-2 py-1 font-mono text-xs text-gray-100 placeholder-gray-600 focus:border-blue-500 focus:outline-none"
                      onkeydown={(e) => { if (e.key === 'Enter') handleCreateBranch(); if (e.key === 'Escape') showNewBranch = false; }}
                    />
                    <button
                      onclick={handleCreateBranch}
                      class="shrink-0 rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-500"
                    >Create</button>
                  </div>
                {:else}
                  <button
                    onclick={() => (showNewBranch = true)}
                    class="w-full rounded px-2 py-1 text-xs text-gray-400 transition hover:bg-gray-800 hover:text-gray-200"
                  >+ New Branch</button>
                {/if}
              </div>

              {#if confirmDeleteBranch}
                <div class="border-t border-red-900/50 bg-red-900/20 p-2">
                  <p class="mb-2 text-xs text-red-300">Delete <span class="font-mono">{confirmDeleteBranch}</span>?</p>
                  <div class="flex gap-2">
                    <button onclick={() => handleDeleteBranch(confirmDeleteBranch!)} class="rounded bg-red-600 px-2 py-1 text-xs text-white hover:bg-red-500">Delete</button>
                    <button onclick={() => (confirmDeleteBranch = null)} class="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700">Cancel</button>
                  </div>
                </div>
              {/if}

              {#if confirmMergeBranch}
                <div class="border-t border-amber-900/50 bg-amber-900/20 p-2">
                  <p class="mb-2 text-xs text-amber-300">Merge <span class="font-mono">{confirmMergeBranch}</span> into current?</p>
                  <div class="flex gap-2">
                    <button onclick={() => handleMergeBranch(confirmMergeBranch!)} class="rounded bg-amber-600 px-2 py-1 text-xs text-white hover:bg-amber-500">Merge</button>
                    <button onclick={() => (confirmMergeBranch = null)} class="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700">Cancel</button>
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <span class="rounded-full px-2 py-0.5 text-xs font-medium {statusBg}">{statusLabel}</span>
      {/if}
    </div>

    <div class="flex items-center gap-1.5">
      <button
        onclick={handleFetch}
        disabled={fetching}
        class="flex items-center gap-1 rounded-lg bg-gray-800 px-2.5 py-1.5 text-xs text-gray-300 transition hover:bg-gray-700 disabled:opacity-50"
        title="Fetch"
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        {fetching ? '...' : 'Fetch'}
      </button>
      <button
        onclick={handlePull}
        disabled={pulling}
        class="flex items-center gap-1 rounded-lg bg-gray-800 px-2.5 py-1.5 text-xs text-gray-300 transition hover:bg-gray-700 disabled:opacity-50"
        title="Pull"
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
        </svg>
        {pulling ? '...' : 'Pull'}
      </button>
      <button
        onclick={handlePush}
        disabled={pushing}
        class="flex items-center gap-1 rounded-lg bg-gray-800 px-2.5 py-1.5 text-xs text-gray-300 transition hover:bg-gray-700 disabled:opacity-50"
        title="Push"
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
        </svg>
        {pushing ? '...' : 'Push'}
      </button>
      <button
        onclick={handleStashSave}
        disabled={stashing}
        class="flex items-center gap-1 rounded-lg bg-gray-800 px-2.5 py-1.5 text-xs text-gray-300 transition hover:bg-gray-700 disabled:opacity-50"
        title="Stash"
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
        </svg>
        {stashing ? '...' : 'Stash'}
      </button>
      <button
        onclick={onRefresh}
        class="rounded-lg bg-gray-800 px-2 py-1.5 text-gray-300 transition hover:bg-gray-700"
        title="Refresh"
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      </button>
    </div>
  </div>

  <div class="flex border-b border-gray-800">
    <button
      class="px-4 py-2 text-sm transition {activeTab === 'overview' ? 'border-b-2 border-blue-500 text-blue-400' : 'text-gray-500 hover:text-gray-300'}"
      onclick={() => (activeTab = 'overview')}
    >
      Overview
    </button>
    <button
      class="px-4 py-2 text-sm transition {activeTab === 'changes' ? 'border-b-2 border-blue-500 text-blue-400' : 'text-gray-500 hover:text-gray-300'}"
      onclick={() => (activeTab = 'changes')}
    >
      Changes
      {#if status && (status.staged_count + status.unstaged_count + status.untracked_count) > 0}
        <span class="ml-1 rounded-full bg-amber-500/20 px-1.5 py-0.5 text-xs text-amber-400">
          {status.staged_count + status.unstaged_count + status.untracked_count}
        </span>
      {/if}
    </button>
    <button
      class="px-4 py-2 text-sm transition {activeTab === 'commits' ? 'border-b-2 border-blue-500 text-blue-400' : 'text-gray-500 hover:text-gray-300'}"
      onclick={() => (activeTab = 'commits')}
    >
      Commits
    </button>
  </div>

  <div class="flex-1 overflow-hidden">
    {#if activeTab === 'overview' && status}
      <div class="overflow-y-auto p-5">
        <div class="grid grid-cols-4 gap-3">
          <div class="rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
            <div class="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500">Ahead</div>
            <span class="text-sm {status.ahead > 0 ? 'font-medium text-blue-400' : status.ahead < 0 ? 'text-gray-600' : 'text-gray-400'}">
              {status.ahead < 0 ? '?' : status.ahead}
            </span>
          </div>
          <div class="rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
            <div class="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500">Behind</div>
            <span class="text-sm {status.behind > 0 ? 'font-medium text-red-400' : status.behind < 0 ? 'text-gray-600' : 'text-gray-400'}">
              {status.behind < 0 ? '?' : status.behind}
            </span>
          </div>
          <div class="rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
            <div class="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500">Staged</div>
            <span class="text-sm {status.staged_count > 0 ? 'font-medium text-green-400' : 'text-gray-400'}">
              {status.staged_count}
            </span>
          </div>
          <div class="rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
            <div class="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500">Unstaged</div>
            <span class="text-sm {status.unstaged_count + status.untracked_count > 0 ? 'font-medium text-amber-400' : 'text-gray-400'}">
              {status.unstaged_count + status.untracked_count}
            </span>
          </div>
        </div>

        {#if Object.keys(status.remote_urls).length > 0}
          <div class="mt-4 rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
            <div class="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500">Remotes</div>
            {#each Object.entries(status.remote_urls) as [name, url]}
              <div class="flex items-center gap-2 text-sm">
                <span class="font-medium text-gray-400">{name}</span>
                <span class="truncate font-mono text-xs text-gray-500">{url}</span>
              </div>
            {/each}
          </div>
        {/if}

        <div class="mt-4 rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
          <div class="mb-3 flex items-center justify-between">
            <div class="text-xs font-medium uppercase tracking-wider text-gray-500">Recent Commits</div>
          </div>
          <div class="space-y-2">
            {#each commits.slice(0, 5) as commit}
              <div class="flex items-start gap-3 py-1">
                <span class="shrink-0 font-mono text-xs text-blue-400">{commit.short_hash}</span>
                <div class="min-w-0 flex-1">
                  <div class="truncate text-sm font-medium">{commit.message}</div>
                  <div class="text-xs text-gray-500">
                    {commit.author} &middot; {formatRelativeTime(commit.timestamp)}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <div class="mt-4 rounded-lg border border-gray-800 bg-gray-900 px-4 py-3">
          <button
            onclick={() => (showStashSection = !showStashSection)}
            class="flex w-full items-center justify-between text-xs font-medium uppercase tracking-wider text-gray-500"
          >
            <span>Stash</span>
            <span>{showStashSection ? '▲' : '▼'}</span>
          </button>
          {#if showStashSection}
            <div class="mt-2 space-y-1">
              {#if stashes.length > 0}
                {#each stashes as stash}
                  <div class="flex items-center gap-2 rounded px-2 py-1 text-sm text-gray-300 hover:bg-gray-800">
                    <span class="font-mono text-xs text-gray-500">stash@{stash.index}</span>
                    <span class="flex-1 truncate text-xs">{stash.message}</span>
                    <button
                      onclick={() => handleStashApply(stash.index)}
                      class="text-xs text-gray-600 transition hover:text-green-400"
                      title="Apply"
                    >Apply</button>
                    <button
                      onclick={() => handleStashDrop(stash.index)}
                      class="text-xs text-gray-600 transition hover:text-red-400"
                      title="Drop"
                    >×</button>
                  </div>
                {/each}
              {:else}
                <div class="py-2 text-xs text-gray-600">No stashes</div>
              {/if}
              <div class="flex gap-2 pt-1">
                <button
                  onclick={handleStashSave}
                  disabled={stashing}
                  class="rounded bg-gray-800 px-2 py-1 text-xs text-gray-400 transition hover:bg-gray-700 hover:text-gray-200 disabled:opacity-50"
                >Stash</button>
                <button
                  onclick={handleStashPop}
                  disabled={popping || stashes.length === 0}
                  class="rounded bg-gray-800 px-2 py-1 text-xs text-gray-400 transition hover:bg-gray-700 hover:text-gray-200 disabled:opacity-50"
                >Pop</button>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else if activeTab === 'changes'}
      <div class="flex h-full flex-col">
        <div class="flex-1 overflow-hidden">
          <ChangedFiles repoPath={repo.path} onRefresh={onRefresh} onToast={onToast} />
        </div>
        {#if showCommit}
          <CommitDialog
            repoPath={repo.path}
            onCommit={handleCommitDone}
            onCancel={() => (showCommit = false)}
            onToast={onToast}
          />
        {:else}
          <div class="border-t border-gray-800 p-3">
            <button
              onclick={() => (showCommit = true)}
              class="w-full rounded-lg bg-blue-600 py-2 text-sm font-medium text-white transition hover:bg-blue-500"
            >
              Commit
            </button>
          </div>
        {/if}
      </div>
    {:else if activeTab === 'commits'}
      <div class="overflow-y-auto p-5">
        <div class="space-y-1">
          {#each commits as commit}
            <div>
              <button
                class="flex w-full items-start gap-3 rounded-lg px-3 py-2 text-left transition hover:bg-gray-900"
                onclick={() => handleExpandCommit(commit.hash)}
              >
                <span class="mt-0.5 shrink-0 {commit.hash === expandedCommit ? 'h-2.5 w-2.5 rounded-full border-2 border-blue-400' : 'h-2 w-2 rounded-full bg-gray-600'}"></span>
                <span class="shrink-0 font-mono text-xs text-blue-400">{commit.short_hash}</span>
                <div class="min-w-0 flex-1">
                  <div class="truncate text-sm font-medium">{commit.message}</div>
                  <div class="text-xs text-gray-500">
                    {commit.author} &middot; {formatRelativeTime(commit.timestamp)}
                  </div>
                </div>
              </button>
              {#if commit.hash === expandedCommit}
                <div class="ml-9 border-l-2 border-gray-800 pl-4 pb-2">
                  {#if commitDetail}
                    <div class="space-y-1">
                      {#each commitDetail.files as file}
                        <div class="flex items-center gap-2 text-xs">
                          <span class="shrink-0 font-mono text-blue-400">{file.path}</span>
                          <span class="text-green-400">+{file.additions}</span>
                          <span class="text-red-400">-{file.deletions}</span>
                          <span class="text-gray-600">{file.status}</span>
                        </div>
                      {/each}
                    </div>
                  {:else}
                    <div class="text-xs text-gray-600">Loading...</div>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

{#if showBranchMenu}
  <div class="fixed inset-0 z-40" role="button" tabindex="-1" onclick={() => { showBranchMenu = false; showNewBranch = false; confirmDeleteBranch = null; confirmMergeBranch = null; }} onkeydown={() => { showBranchMenu = false; }}></div>
{/if}
