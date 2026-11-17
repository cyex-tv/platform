<script lang="ts" generics="T">
  // The three states of a loaded page: loading, a message with Try again, and the content.
  import type { Snippet } from 'svelte';
  import type { Loader } from './loader.svelte';
  import State from './State.svelte';

  let { source, label = 'Loading…', children }: { source: Loader<T>; label?: string; children: Snippet<[T]> } = $props();
</script>

{#if source.data !== null}
  {@render children(source.data)}
{:else if source.error}
  <State kind="error" title="That didn’t load">
    {source.error}
    {#snippet actions()}
      <button class="ui-btn secondary" type="button" disabled={source.loading} onclick={() => source.reload()}>Try again</button>
    {/snippet}
  </State>
{:else}
  <State kind="loading" title={label} />
{/if}
