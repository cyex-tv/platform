<script lang="ts">
  // The head of an admin page, the browse page's way: the one page title with its count beside it
  // on the baseline ("Badges  12 badges"), the page's actions at the right, and under it one slim
  // toolbar row (tabs, search, filters, sort) over the list. No sentence of explanation: the
  // list says what the page is. An optional back link sits above the title.
  //   <Bar title="Badges" count="12 badges">{#snippet tools()}…{/snippet}{#snippet actions()}…{/snippet}</Bar>
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  type Props = {
    title: string;
    /** The count or status beside the title (mono numbers), text or markup. */
    count?: string | Snippet | null;
    back?: { href: string; label: string };
    actions?: Snippet;
    /** The toolbar row under the title: tabs, search, filters, sort. */
    tools?: Snippet;
    /** The title is a name (an account's, a role's): keep its direction. */
    name?: boolean;
  };
  let { title, count = null, back, actions, tools, name = false }: Props = $props();
</script>

<header class="bar">
  {#if back}<a class="back" href={back.href}><Icon name="arrow-left" size={14} />{back.label}</a>{/if}
  <div class="row">
    <div class="t">
      <h1 class:name>{#if name}<bdi>{title}</bdi>{:else}{title}{/if}</h1>
      {#if count}<p class="c" aria-live="polite">{#if typeof count === 'string'}{count}{:else}{@render count()}{/if}</p>{/if}
    </div>
    {#if actions}<div class="acts">{@render actions()}</div>{/if}
  </div>
  {#if tools}<div class="adm-toolbar tools">{@render tools()}</div>{/if}
</header>

<style>
  .bar { display: grid; gap: 6px; margin-bottom: 12px; min-width: 0; }
  .back { display: inline-flex; align-items: center; gap: 6px; width: fit-content; min-height: 28px; font-size: var(--t-small); font-weight: 600; color: var(--muted); text-decoration: none; }
  .row { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px 16px; min-height: 40px; }
  .t { display: flex; align-items: baseline; gap: 12px; min-width: 0; }
  h1 { margin: 0; font-size: var(--t-page); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; min-width: 0; overflow-wrap: anywhere; }
  h1.name { overflow: clip; }
  .c { margin: 0; font-family: var(--font-mono); font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .c :global(b) { color: var(--text); font-weight: 600; }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .tools { border-bottom: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
</style>
