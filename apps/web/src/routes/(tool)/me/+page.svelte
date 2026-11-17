<script lang="ts">
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import type { MyEmotesQuery } from '$lib/gql/graphql';
  import { MY_EMOTES } from '$lib/queries';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import { STATE_LABEL } from '$lib/tool/state';

  const PER_PAGE = 60;
  const current = $derived(Math.max(1, Number.parseInt(page.url.searchParams.get('page') ?? '1', 10) || 1));
  let result = $state<MyEmotesQuery['myEmotes'] | null>(null);
  let problem = $state<string | null>(null);

  $effect(() => {
    const p = current;
    problem = null;
    gql(MY_EMOTES, { page: p, perPage: PER_PAGE })
      .then((r) => (result = r.myEmotes))
      .catch((e) => (problem = e.message));
  });

  // Only what needs attention gets a label; approved emotes are the normal case.
  const states = $derived(
    Object.fromEntries(
      (result?.items ?? []).filter((e) => e.state !== 'APPROVED').map((e) => [e.id, STATE_LABEL[e.state].toLowerCase()]),
    ),
  );
  const pages = $derived(result ? Math.max(1, Math.ceil(result.total / PER_PAGE)) : 1);
</script>

<svelte:head><title>Your emotes - cyex.tv</title></svelte:head>

<div class="ui-head">
  <div>
    <h1>Your emotes</h1>
    <p>{result ? `${count(result.total)} ${result.total === 1 ? 'emote' : 'emotes'}` : ' '}</p>
  </div>
  <div class="ui-actions">
    <a class="ui-btn primary" href="/me/upload">
      <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true"><path d="M12 16V4M7 9l5-5 5 5M5 20h14" /></svg>
      Upload
    </a>
  </div>
</div>

{#if problem}
  <p class="ui-error" role="alert">{problem}</p>
{:else if !result}
  <EmoteGrid items={[]} loading skeletons={12} creator={false} labels={false} />
{:else if result.items.length === 0}
  <div class="ui-empty">
    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M8 14s1.5 2 4 2 4-2 4-2M9 9h.01M15 9h.01" /></svg>
    <p>No emotes yet. Upload one and it shows up here.</p>
    <a class="ui-btn secondary" href="/me/upload">Upload an emote</a>
  </div>
{:else}
  <EmoteGrid
    items={result.items}
    creator={false}
    eager={12}
    label="Your emotes"
    href={(e) => `/me/emotes/${e.id}`}
    note={(e) => (e.id in states ? states[e.id] : undefined)}
  />
  {#if pages > 1}
    <nav class="paging ui-actions" aria-label="Pages">
      {#if current > 1}<a class="ui-btn secondary small" href="?page={current - 1}">Previous</a>{/if}
      <span class="ui-hint ui-mono">{current} / {pages}</span>
      {#if current < pages}<a class="ui-btn secondary small" href="?page={current + 1}">Next</a>{/if}
    </nav>
  {/if}
{/if}

<style>
  .paging { justify-content: center; margin-top: 20px; }
</style>
