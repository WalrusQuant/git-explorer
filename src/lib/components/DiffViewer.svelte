<script lang="ts">
  import type { FileDiff } from '../types';

  let { diff }: { diff: FileDiff | null } = $props();
</script>

<div class="h-full overflow-y-auto">
  {#if diff}
    {#each diff.hunks as hunk, i}
      <div class="border-b border-gray-800/30">
        <div class="bg-gray-800/50 px-3 py-1 font-mono text-xs text-gray-500">
          @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
        </div>
        {#each hunk.lines as line}
          <div
            class="flex font-mono text-xs leading-5 {line.origin === 'add'
              ? 'bg-diff-add-bg text-diff-add-text'
              : line.origin === 'remove'
                ? 'bg-diff-remove-bg text-diff-remove-text'
                : 'text-gray-400'}"
          >
            <span class="w-10 shrink-0 select-none text-right text-gray-600"
              >{line.old_line_no >= 0 ? line.old_line_no : ''}</span
            >
            <span class="w-10 shrink-0 select-none text-right text-gray-600"
              >{line.new_line_no >= 0 ? line.new_line_no : ''}</span
            >
            <span
              class="w-4 shrink-0 select-none text-center {line.origin === 'add'
                ? 'text-diff-add-marker'
                : line.origin === 'remove'
                  ? 'text-diff-remove-marker'
                  : 'text-gray-700'}"
              >{line.origin === 'add' ? '+' : line.origin === 'remove' ? '−' : ' '}</span
            >
            <pre class="flex-1 whitespace-pre-wrap break-all">{line.content}</pre>
          </div>
        {/each}
      </div>
    {/each}
  {:else}
    <div class="flex h-full items-center justify-center text-sm text-gray-600"
      >Select a file to view its diff</div
    >
  {/if}
</div>
