<script lang="ts" generics="T">
  // The three states of a loaded page: loading, the one error pattern (Problem), and the content.
  import type { Snippet } from 'svelte';
  import type { Loader } from './loader.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';

  type Props = {
    source: Loader<T>;
    label?: string;
    children: Snippet<[T]>;
    /** The one link out of an error; the front page when left out. */
    home?: { href: string; label: string };
    /** What's missing, when the thing asked for isn't there: its headline and sentence. */
    missing?: { title: string; body: string };
    /** The error is the whole page (an h1), not a block inside one. */
    level?: 1 | 2;
  };
  let { source, label = 'Loading…', children, home, missing, level = 2 }: Props = $props();
</script>

{#if source.data !== null}
  {@render children(source.data)}
{:else if source.error}
  <Problem cause={source.cause ?? 'server'} {home} {level} title={missing?.title} body={missing?.body} onretry={() => source.reload()} />
{:else}
  <State kind="loading" title={label} />
{/if}
