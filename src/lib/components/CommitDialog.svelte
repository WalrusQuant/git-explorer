<script lang="ts">
  import type { ToastMessage } from '../types';
  import { invoke } from '@tauri-apps/api/core';

  let {
    repoPath,
    onCommit,
    onCancel,
    onToast,
  }: {
    repoPath: string;
    onCommit: () => void;
    onCancel: () => void;
    onToast: (msg: string, type: ToastMessage['type']) => void;
  } = $props();

  const prefixes = ['feat', 'fix', 'chore', 'docs', 'refactor', 'test', 'style'];

  let message: string = $state('');
  let selectedPrefix: string = $state('');
  let committing: boolean = $state(false);
  let error: string | null = $state(null);

  let charCount = $derived(message.length);
  let fullMessage = $derived(
    selectedPrefix && !message.startsWith(selectedPrefix + ':')
      ? `${selectedPrefix}: ${message}`
      : message
  );

  async function handleCommit() {
    if (!fullMessage.trim()) return;
    committing = true;
    error = null;
    try {
      await invoke('git_commit', { path: repoPath, message: fullMessage.trim() });
      message = '';
      selectedPrefix = '';
      onToast('Committed successfully', 'success');
      onCommit();
    } catch (e) {
      error = String(e);
      onToast(String(e), 'error');
    }
    committing = false;
  }

  $effect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleCommit();
      } else if (e.key === 'Escape') {
        onCancel();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  });
</script>

<form class="border-t border-gray-800 bg-gray-900 p-4" onsubmit={(e) => { e.preventDefault(); handleCommit(); }}>
  {#if error}
    <div class="mb-3 rounded-lg border border-red-800 bg-red-900/30 px-3 py-2 text-xs text-red-300">
      {error}
    </div>
  {/if}
  <div class="mb-2 flex items-center gap-2">
    <div class="relative">
      <select
        bind:value={selectedPrefix}
        class="appearance-none rounded-lg border border-gray-700 bg-gray-800 px-2 py-1.5 pr-6 text-xs text-gray-300 focus:border-blue-500 focus:outline-none"
      >
        <option value="">prefix</option>
        {#each prefixes as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
      <svg class="pointer-events-none absolute right-1.5 top-1/2 h-3 w-3 -translate-y-1/2 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </div>
    <span class="ml-auto text-xs {charCount > 72 ? 'text-red-400' : charCount > 50 ? 'text-amber-400' : 'text-gray-600'}">
      {charCount}/72
    </span>
  </div>
  <textarea
    bind:value={message}
    placeholder="Commit message... (Cmd+Enter to commit)"
    rows="3"
    class="mb-3 w-full resize-none rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100 placeholder-gray-600 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
  ></textarea>
  <div class="flex items-center justify-between gap-2">
    <span class="text-xs text-gray-600">
      {#if selectedPrefix && !message.startsWith(selectedPrefix + ':')}
        <span class="text-gray-400">{selectedPrefix}:</span> {message}
      {/if}
    </span>
    <div class="flex items-center gap-2">
      <button
        onclick={onCancel}
        class="rounded-lg px-3 py-1.5 text-sm text-gray-400 transition hover:text-gray-200"
      >
        Cancel
      </button>
      <button
        onclick={handleCommit}
        disabled={!fullMessage.trim() || committing}
        class="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white transition hover:bg-blue-500 disabled:opacity-50"
      >
        {committing ? 'Committing...' : 'Commit'}
      </button>
    </div>
  </div>
</form>
