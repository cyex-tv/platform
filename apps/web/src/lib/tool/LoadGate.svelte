<script lang="ts" generics="T">
  // The three states of a loaded page: loading, a message with Try again, and the content.
  import type { Snippet } from 'svelte';
  import type { Loader } from './loader.svelte';

  let { source, label = 'Loading…', children }: { source: Loader<T>; label?: string; children: Snippet<[T]> } = $props();
</script>

{#if source.data !== null}
  {@render children(source.data)}
{:else if source.error}
  <div class="ui-empty" role="alert">
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
    <p>{source.error}</p>
    <button class="ui-btn secondary" type="button" disabled={source.loading} onclick={() => source.reload()}>Try again</button>
  </div>
{:else}
  <div class="ui-empty" role="status" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span><p>{label}</p></div>
{/if}
