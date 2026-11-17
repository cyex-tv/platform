<script lang="ts">
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import type { MyEmotesQuery } from '$lib/gql/graphql';
  import { MY_EMOTES } from '$lib/queries';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import State from '$lib/components/State.svelte';
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

<PageHead title="Your emotes" line={result ? `${count(result.total)} ${result.total === 1 ? 'emote' : 'emotes'}. Pending ones are only visible to you until a moderator looks at them.` : ' '}>
  {#snippet actions()}
    <a class="ui-btn primary" href="/me/upload"><Icon name="upload" size={15} />Upload</a>
  {/snippet}
</PageHead>

{#if problem}
  <State kind="error" title="Your emotes didn’t load">
    {problem}
    {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={() => location.reload()}>Try again</button>{/snippet}
  </State>
{:else if !result}
  <EmoteGrid items={[]} loading skeletons={12} creator={false} labels={false} />
{:else if result.items.length === 0}
  <State title="No emotes yet">
    Upload one and it shows up here, ready to add to a set.
    {#snippet actions()}<a class="ui-btn primary" href="/me/upload"><Icon name="upload" size={15} />Upload an emote</a>{/snippet}
  </State>
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
    <nav class="paging" aria-label="Pages">
      {#if current > 1}<a class="ui-btn secondary small" href="?page={current - 1}">Previous</a>{/if}
      <span class="ui-hint ui-num">{current} / {pages}</span>
      {#if current < pages}<a class="ui-btn secondary small" href="?page={current + 1}">Next</a>{/if}
    </nav>
  {/if}
{/if}

<style>
  .paging { display: flex; align-items: center; justify-content: center; gap: 12px; margin-top: 24px; }
</style>
