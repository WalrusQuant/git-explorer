<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { TreeNode, Commit, Config, ToastMessage } from './lib/types';
  import { initTheme, toggleTheme, getTheme } from './lib/theme';
  import FolderTree from './lib/components/FolderTree.svelte';
  import RepoDetail from './lib/components/RepoDetail.svelte';
  import SearchBar from './lib/components/SearchBar.svelte';
  import Toast from './lib/components/Toast.svelte';

  let config: Config = $state({ root_path: '' });
  let theme: 'dark' | 'light' = $state('dark');
  let treeNodes: TreeNode[] = $state([]);
  let selectedPath: string | null = $state(null);
  let searchQuery: string = $state('');
  let isScanning: boolean = $state(false);
  let commitLog: Commit[] = $state([]);
  let error: string | null = $state(null);
  let newRootPath: string = $state('');
  let toasts: ToastMessage[] = $state([]);
  let toastId: number = 0;
  let latestRequestId: number = 0;

  let selectedRepo: TreeNode | null = $derived(
    treeNodes.find((n) => n.path === selectedPath) ?? null
  );

  // M6: single-pass filteredNodes derivation
  let filteredNodes: TreeNode[] = $derived.by(() => {
    if (!searchQuery.trim()) return treeNodes;
    const q = searchQuery.toLowerCase();
    const matchingPaths = new Set<string>();

    function addAncestors(path: string) {
      let p = path.substring(0, path.lastIndexOf('/'));
      while (p) {
        if (matchingPaths.has(p)) break; // already added this and its ancestors
        matchingPaths.add(p);
        const next = p.substring(0, p.lastIndexOf('/'));
        if (next === p) break;
        p = next;
      }
    }

    function addDescendants(path: string) {
      for (const other of treeNodes) {
        if (other.path.startsWith(path + '/')) {
          matchingPaths.add(other.path);
        }
      }
    }

    for (const node of treeNodes) {
      if (node.name.toLowerCase().includes(q)) {
        matchingPaths.add(node.path);
        addAncestors(node.path);
        if (node.is_repo) {
          addDescendants(node.path);
        }
      }
    }

    return treeNodes.filter((n) => matchingPaths.has(n.path));
  });

  function showToast(message: string, type: ToastMessage['type'] = 'info') {
    const id = ++toastId;
    toasts = [...toasts, { id, message, type }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
    }, 3000);
  }

  function handleToggleTheme() {
    toggleTheme();
    theme = getTheme();
  }

  async function loadConfig() {
    try {
      config = await invoke<Config>('load_config');
      newRootPath = config.root_path;
      if (config.root_path) {
        await handleScan();
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function handleScan() {
    if (!config.root_path) return;
    isScanning = true;
    error = null;
    try {
      treeNodes = await invoke<TreeNode[]>('scan_directory', { root: config.root_path });
    } catch (e) {
      error = String(e);
      treeNodes = [];
    }
    isScanning = false;
  }

  async function handleRefresh() {
    await handleScan();
    if (selectedPath) {
      await loadRepoDetails(selectedPath);
    }
  }

  // H5: cancellation token — discard stale responses when selection changes
  async function loadRepoDetails(path: string) {
    const id = ++latestRequestId;
    try {
      const result = await invoke<Commit[]>('get_commit_log', { path, limit: 30 });
      if (id !== latestRequestId) return;
      commitLog = result;
    } catch {
      if (id !== latestRequestId) return;
      commitLog = [];
    }
  }

  async function handleSaveConfig() {
    if (!newRootPath.trim()) return;
    try {
      await invoke('save_config', { rootPath: newRootPath.trim() });
      config.root_path = newRootPath.trim();
      await handleScan();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleBrowse() {
    const selected = await open({ directory: true, multiple: false, title: 'Select root directory' });
    if (selected && typeof selected === 'string') {
      newRootPath = selected;
    }
  }

  function handleSelectRepo(path: string) {
    selectedPath = path;
  }

  $effect(() => {
    if (selectedPath) {
      loadRepoDetails(selectedPath);
    }
  });
  $effect(() => {
    initTheme();
    theme = getTheme();
  });
  $effect(() => {
    loadConfig();
  });
</script>

<Toast {toasts} />

<div class="flex h-screen bg-gray-950 text-gray-100">
  <aside class="flex w-80 flex-col border-r border-gray-800 bg-gray-900">
    <div class="border-b border-gray-800 p-3">
      <SearchBar onSearch={(q) => (searchQuery = q)} />
    </div>

    <div class="flex-1 overflow-y-auto p-2">
      {#if isScanning}
        <div class="flex flex-col items-center justify-center gap-3 py-12">
          <div
            class="h-8 w-8 animate-spin rounded-full border-2 border-blue-500 border-t-transparent"
          ></div>
          <span class="text-sm text-gray-400">Scanning repositories...</span>
        </div>
      {:else if filteredNodes.length > 0}
        <FolderTree
          nodes={filteredNodes}
          selectedPath={selectedPath}
          onSelect={handleSelectRepo}
        />
      {:else if config.root_path}
        <div class="p-4 text-center text-sm text-gray-500">
          No git repositories found in
          <span class="font-mono text-gray-400">{config.root_path}</span>
        </div>
      {:else}
        <div class="p-4 text-center text-sm text-gray-500">
          Configure a root path to get started
        </div>
      {/if}
    </div>

    <div class="border-t border-gray-800 p-3">
      <div class="mb-2 flex items-center justify-between">
        <span class="truncate font-mono text-xs text-gray-500">
          {config.root_path || 'No root path set'}
        </span>
        <div class="flex items-center gap-2">
          <button
            onclick={handleToggleTheme}
            class="text-gray-500 transition hover:text-gray-300"
            title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
          >
            {#if theme === 'dark'}
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
              </svg>
            {:else}
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
              </svg>
            {/if}
          </button>
          <button
            onclick={async () => { await handleBrowse(); if (newRootPath) await handleSaveConfig(); }}
            class="shrink-0 text-xs text-blue-400 hover:text-blue-300"
          >
            Change
          </button>
        </div>
      </div>
      <button
        onclick={handleRefresh}
        disabled={isScanning || !config.root_path}
        class="flex w-full items-center justify-center gap-2 rounded-lg bg-gray-800 px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-gray-700 disabled:opacity-50"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
        Refresh
      </button>
    </div>
  </aside>

  <main class="flex-1 overflow-y-auto">
    {#if error}
      <div
        class="m-4 flex items-center justify-between rounded-lg border border-red-800 bg-red-900/30 px-4 py-3 text-sm text-red-300"
      >
        <span>{error}</span>
        <button onclick={() => (error = null)} class="ml-4 font-bold text-red-400 hover:text-red-200"
          >&times;</button
        >
      </div>
    {/if}

    {#if !config.root_path}
      <div class="flex h-full items-center justify-center">
        <div class="w-full max-w-md rounded-xl border border-gray-800 bg-gray-900 p-8">
          <h2 class="mb-2 text-xl font-semibold">Welcome to Git Explorer</h2>
          <p class="mb-6 text-sm text-gray-400"
            >Set a root directory to scan for git repositories.</p
          >
          <div class="mb-4">
            <label for="root-path" class="mb-1 block text-sm font-medium text-gray-300">Root Path</label>
            <div class="flex gap-2">
              <input
                id="root-path"
                type="text"
                bind:value={newRootPath}
                placeholder="/Users/you/Code"
                class="w-full rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 font-mono text-sm text-gray-100 placeholder-gray-600 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
              <button
                onclick={handleBrowse}
                class="shrink-0 rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm font-medium text-gray-300 transition hover:bg-gray-700"
              >
                Browse
              </button>
            </div>
          </div>
          <button
            onclick={handleSaveConfig}
            class="w-full rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-500"
          >
            Save &amp; Scan
          </button>
        </div>
      </div>
    {:else if selectedRepo}
      <RepoDetail repo={selectedRepo} commits={commitLog} onRefresh={handleRefresh} onToast={showToast} />
    {:else}
      <div class="flex h-full items-center justify-center text-gray-500">
        <div class="text-center">
          <svg
            class="mx-auto mb-4 h-12 w-12 text-gray-700"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="1.5"
              d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
            />
          </svg>
          <p class="text-sm">Select a repository to view details</p>
        </div>
      </div>
    {/if}
  </main>
</div>
