<script lang="ts">
  import type { ToastMessage } from '../types';

  let { toasts }: { toasts: ToastMessage[] } = $props();

  function bgClass(type: ToastMessage['type']): string {
    switch (type) {
      case 'success': return 'bg-green-900/80 border-green-700 text-green-200';
      case 'error': return 'bg-red-900/80 border-red-700 text-red-200';
      case 'info': return 'bg-blue-900/80 border-blue-700 text-blue-200';
      default: return 'bg-gray-800 border-gray-700 text-gray-200';
    }
  }
</script>

<div class="pointer-events-none fixed right-4 top-4 z-50 flex flex-col gap-2">
  {#each toasts as toast (toast.id)}
    <div
      class="pointer-events-auto rounded-lg border px-4 py-2.5 text-sm shadow-lg transition-all {bgClass(toast.type)}"
    >
      {toast.message}
    </div>
  {/each}
</div>
