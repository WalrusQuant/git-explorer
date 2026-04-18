<script lang="ts">
  import type { TreeNode } from '../types';

  let {
    nodes,
    selectedPath,
    onSelect,
  }: {
    nodes: TreeNode[];
    selectedPath: string | null;
    onSelect: (path: string) => void;
  } = $props();

  let expandedPaths: Set<string> = $state(new Set());

  function parentPath(path: string): string {
    const idx = path.lastIndexOf('/');
    if (idx <= 0) return '/';
    return path.substring(0, idx);
  }

  let repoCountByDir: Map<string, number> = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const node of nodes) {
      if (node.is_repo) {
        let p = node.path;
        while (p && p !== '/') {
          const parent = parentPath(p);
          if (parent === p) break;
          counts.set(parent, (counts.get(parent) ?? 0) + 1);
          p = parent;
        }
      }
    }
    return counts;
  });

  function toggleExpand(path: string) {
    const next = new Set(expandedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    expandedPaths = next;
  }

  function isNodeVisible(node: TreeNode): boolean {
    if (node.depth === 0) return true;
    const parent = parentPath(node.path);
    return expandedPaths.has(parent);
  }

  function statusDotColor(status: string | null): string {
    switch (status) {
      case 'clean':
        return 'bg-green-500';
      case 'dirty':
        return 'bg-amber-500';
      case 'ahead':
        return 'bg-blue-500';
      case 'behind':
        return 'bg-red-500';
      case 'diverged':
        return 'bg-orange-500';
      case 'no-remote':
        return 'bg-gray-500';
      case 'error':
        return 'bg-red-700';
      default:
        return 'bg-gray-600';
    }
  }
</script>

<div class="select-none">
  {#each nodes as node (node.path)}
    {#if isNodeVisible(node)}
      <button
        class="flex w-full items-center gap-1.5 rounded-md px-1 py-1 text-left text-sm transition hover:bg-gray-800 {node.path ===
        selectedPath
          ? 'bg-blue-600/20 text-blue-300'
          : 'text-gray-300'}"
        style="padding-left: {node.depth * 16 + 4}px"
        onclick={() => {
          if (node.is_repo) {
            onSelect(node.path);
          } else if (node.has_repo_descendant) {
            toggleExpand(node.path);
          }
        }}
      >
        {#if node.has_repo_descendant && !node.is_repo}
          <span class="w-4 shrink-0 text-center text-gray-600 text-xs">
            {expandedPaths.has(node.path) ? '▼' : '▶'}
          </span>
        {:else}
          <span class="w-4 shrink-0"></span>
        {/if}

        {#if node.is_repo}
          <svg class="h-4 w-4 shrink-0 text-blue-400" viewBox="0 0 16 16" fill="currentColor">
            <path
              d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
            />
          </svg>
        {:else if node.has_repo_descendant}
          <svg class="h-4 w-4 shrink-0 text-blue-400/60" viewBox="0 0 16 16" fill="currentColor">
            <path
              d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
            />
          </svg>
        {:else}
          <svg class="h-4 w-4 shrink-0 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
            />
          </svg>
        {/if}

        <span class="truncate">{node.name}</span>

        {#if node.is_repo && node.repo_status}
          <span
            class="ml-auto h-2 w-2 shrink-0 rounded-full {statusDotColor(node.repo_status.status)}"
          ></span>
        {:else if node.has_repo_descendant && !node.is_repo}
          <span class="ml-auto shrink-0 rounded-full bg-blue-500/20 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-blue-400">
            {repoCountByDir.get(node.path) ?? 0}
          </span>
        {/if}
      </button>
    {/if}
  {/each}
</div>
