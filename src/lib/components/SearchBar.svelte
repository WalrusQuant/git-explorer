<script lang="ts">
  let query: string = $state('');
  let { onSearch }: { onSearch: (query: string) => void } = $props();

  $effect(() => {
    onSearch(query);
  });
</script>

<div class="relative">
  <svg
    class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-500"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
  >
    <path
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
    />
  </svg>
  <input
    type="text"
    bind:value={query}
    placeholder="Filter repos..."
    class="w-full rounded-lg border border-gray-700 bg-gray-800 py-2 pl-9 pr-8 text-sm text-gray-100 placeholder-gray-600 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
  />
  {#if query}
    <button
      onclick={() => (query = '')}
      class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
    >
      &times;
    </button>
  {/if}
</div>
