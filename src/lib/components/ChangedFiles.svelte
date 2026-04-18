<script lang="ts">
  import type { FileChange, FileDiff, ToastMessage } from '../types';
  import { invoke } from '@tauri-apps/api/core';
  import DiffViewer from './DiffViewer.svelte';

  let {
    repoPath,
    onRefresh,
    onToast,
  }: {
    repoPath: string;
    onRefresh: () => void;
    onToast: (msg: string, type: ToastMessage['type']) => void;
  } = $props();

  let files: FileChange[] = $state([]);
  let selectedFile: string | null = $state(null);
  let selectedStaged: boolean | null = $state(null);
  let fileDiff: FileDiff | null = $state(null);
  let loading = $state(false);
  let confirmDiscard: string | null = $state(null);
  let latestFilesRequestId: number = 0;
  let latestDiffRequestId: number = 0;

  let stagedFiles: FileChange[] = $derived(files.filter((f) => f.staged));
  let unstagedFiles: FileChange[] = $derived(files.filter((f) => !f.staged));

  // H6: cancellation token — discard stale responses when repoPath changes
  async function loadFiles() {
    const id = ++latestFilesRequestId;
    loading = true;
    try {
      const result = await invoke<FileChange[]>('get_changed_files', { path: repoPath });
      if (id !== latestFilesRequestId) return;
      files = result;
    } catch {
      if (id !== latestFilesRequestId) return;
      files = [];
    }
    loading = false;
  }

  // H6: cancellation token for diff loads
  async function loadDiff(filePath: string, staged: boolean) {
    const id = ++latestDiffRequestId;
    try {
      const result = await invoke<FileDiff>('get_file_diff', { path: repoPath, filePath, staged });
      if (id !== latestDiffRequestId) return;
      fileDiff = result;
    } catch {
      if (id !== latestDiffRequestId) return;
      fileDiff = null;
    }
  }

  async function handleStage(filePath: string) {
    try {
      await invoke('stage_file', { path: repoPath, filePath });
      await loadFiles();
      onRefresh();
      if (selectedFile === filePath && selectedStaged === false) {
        await loadDiff(filePath, true);
        selectedStaged = true;
      }
    } catch {}
  }

  async function handleUnstage(filePath: string) {
    try {
      await invoke('unstage_file', { path: repoPath, filePath });
      await loadFiles();
      onRefresh();
      if (selectedFile === filePath && selectedStaged === true) {
        selectedStaged = false;
      }
    } catch {}
  }

  async function handleStageAll() {
    try {
      await invoke('stage_all', { path: repoPath });
      await loadFiles();
      onRefresh();
      onToast('All files staged', 'success');
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleUnstageAll() {
    try {
      await invoke('unstage_all', { path: repoPath });
      await loadFiles();
      onRefresh();
      onToast('All files unstaged', 'success');
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  async function handleDiscard(filePath: string) {
    confirmDiscard = null;
    try {
      await invoke('discard_file', { path: repoPath, filePath });
      await loadFiles();
      onRefresh();
      if (selectedFile === filePath) {
        selectedFile = null;
        fileDiff = null;
      }
      onToast('Changes discarded', 'success');
    } catch (e) {
      onToast(String(e), 'error');
    }
  }

  function selectFile(file: FileChange) {
    selectedFile = file.path;
    selectedStaged = file.staged;
    loadDiff(file.path, file.staged);
  }

  function statusIcon(status: string): string {
    switch (status) {
      case 'added': return 'A';
      case 'deleted': return 'D';
      case 'modified': return 'M';
      case 'renamed': return 'R';
      case 'untracked': return 'U';
      default: return '?';
    }
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'added': return 'text-green-400';
      case 'deleted': return 'text-red-400';
      case 'modified': return 'text-amber-400';
      case 'renamed': return 'text-blue-400';
      case 'untracked': return 'text-gray-400';
      default: return 'text-gray-500';
    }
  }

  $effect(() => {
    loadFiles();
  });
</script>

<div class="flex h-full">
  <div class="flex w-72 shrink-0 flex-col border-r border-gray-800">
    {#if stagedFiles.length > 0}
      <div class="flex items-center justify-between border-b border-gray-800/50 px-3 py-2">
        <span class="text-xs font-medium uppercase tracking-wider text-gray-500"
          >Staged ({stagedFiles.length})</span
        >
        <button
          onclick={handleUnstageAll}
          class="text-xs text-gray-500 transition hover:text-gray-300"
          >Unstage All</button
        >
      </div>
      {#each stagedFiles as file (file.path)}
        <div
          class="flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-sm transition hover:bg-gray-800 {selectedFile === file.path && selectedStaged === true ? 'bg-blue-600/20 text-blue-300' : 'text-gray-300'}"
          onclick={() => selectFile(file)}
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') selectFile(file); }}
        >
          <span class="w-3 shrink-0 text-center text-xs font-bold {statusColor(file.status)}"
            >{statusIcon(file.status)}</span
          >
          <span class="truncate font-mono text-xs">{file.path}</span>
          <button
            onclick={(e) => { e.stopPropagation(); handleUnstage(file.path); }}
            class="ml-auto shrink-0 text-xs text-gray-600 transition hover:text-red-400"
            title="Unstage"
            >−</button
          >
        </div>
      {/each}
    {/if}

    {#if unstagedFiles.length > 0}
      <div class="flex items-center justify-between border-b border-gray-800/50 border-t border-gray-800/50 px-3 py-2">
        <span class="text-xs font-medium uppercase tracking-wider text-gray-500"
          >Unstaged ({unstagedFiles.length})</span
        >
        <button
          onclick={handleStageAll}
          class="text-xs text-gray-500 transition hover:text-gray-300"
          >Stage All</button
        >
      </div>
      {#each unstagedFiles as file (file.path)}
        <div
          class="group/file flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-sm transition hover:bg-gray-800 {selectedFile === file.path && selectedStaged === false ? 'bg-blue-600/20 text-blue-300' : 'text-gray-300'}"
          onclick={() => selectFile(file)}
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') selectFile(file); }}
        >
          <span class="w-3 shrink-0 text-center text-xs font-bold {statusColor(file.status)}"
            >{statusIcon(file.status)}</span
          >
          <span class="truncate font-mono text-xs">{file.path}</span>
          <div class="ml-auto flex shrink-0 items-center gap-1">
            {#if confirmDiscard === file.path}
              <button
                onclick={(e) => { e.stopPropagation(); handleDiscard(file.path); }}
                class="text-xs font-medium text-red-400 hover:text-red-300"
                >Confirm</button
              >
            {:else}
              <button
                onclick={(e) => { e.stopPropagation(); confirmDiscard = file.path; }}
                class="text-xs text-gray-600 opacity-0 transition hover:text-red-400 group-hover/file:opacity-100"
                title="Discard changes"
                >×</button
              >
            {/if}
            <button
              onclick={(e) => { e.stopPropagation(); handleStage(file.path); }}
              class="text-xs text-gray-600 transition hover:text-green-400"
              title="Stage"
              >+</button
            >
          </div>
        </div>
      {/each}
    {/if}

    {#if files.length === 0 && !loading}
      <div class="p-4 text-center text-sm text-gray-600">No changes</div>
    {/if}
  </div>

  <div class="flex-1">
    {#if selectedFile && !fileDiff}
      <div class="flex h-full items-center justify-center text-sm text-gray-600">Loading diff...</div>
    {:else}
      <DiffViewer diff={fileDiff} />
    {/if}
  </div>
</div>
