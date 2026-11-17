<script lang="ts">
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import { count } from '$lib/format';

  let { data } = $props();

  const kinds = [
    { value: 'all', label: 'All' },
    { value: 'animated', label: 'Animated' },
    { value: 'still', label: 'Still' },
  ];

  function href(over: { kind?: string; page?: number }) {
    const p = new URLSearchParams();
    if (data.query) p.set('q', data.query);
    const kind = over.kind ?? data.kind;
    if (kind !== 'all') p.set('kind', kind);
    const page = over.page ?? 1;
    if (page > 1) p.set('page', String(page));
    const qs = p.toString();
    return qs ? `/search?${qs}` : '/search';
  }

  const pages = $derived(data.results ? Math.max(1, Math.ceil(data.results.total / data.results.perPage)) : 1);
</script>

<svelte:head>
  <title>{data.query ? `${data.query} - ` : ''}Search - cyex.tv</title>
  <meta name="description" content="Search approved cyex.tv emotes by name and tag." />
</svelte:head>

<div class="page">
  <header class="page-head">
    <h1>Search emotes</h1>
    <p class="lead">Approved emotes by name and tag, the ones most channels use first.</p>
  </header>

  <form class="field" method="GET" action="/search" role="search">
    <label class="sr-only" for="q">Emote name or tag</label>
    <input id="q" class="input" name="q" type="search" value={data.query} placeholder="Emote name or tag" autocomplete="off" />
    {#if data.kind !== 'all'}<input type="hidden" name="kind" value={data.kind} />{/if}
    <button class="btn btn-primary" type="submit">Search</button>
  </form>

  <div class="bar">
    <div class="segmented" role="group" aria-label="Kind">
      {#each kinds as k}
        <a href={href({ kind: k.value })} data-sveltekit-noscroll aria-current={data.kind === k.value ? 'true' : undefined}
          >{k.label}</a
        >
      {/each}
    </div>
    {#if data.results}
      <p class="muted total">
        {count(data.results.total)}
        {data.results.total === 1 ? 'emote' : 'emotes'}
      </p>
    {/if}
  </div>

  {#if data.error}
    <p class="muted" role="status">{data.error}</p>
  {:else if data.results}
    <EmoteGrid
      items={data.results.items}
      empty={data.query ? `No approved emotes match “${data.query}”.` : 'No approved emotes yet.'}
    />
    {#if pages > 1}
      <nav class="pager" aria-label="Pages">
        {#if data.results.page > 1}
          <a class="btn btn-s" href={href({ page: data.results.page - 1 })}>Previous</a>
        {/if}
        <span class="muted">Page {count(data.results.page)} of {count(pages)}</span>
        {#if data.results.page < pages}
          <a class="btn btn-s" href={href({ page: data.results.page + 1 })}>Next</a>
        {/if}
      </nav>
    {/if}
  {/if}
</div>

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px; margin: 24px 0 20px; }
  .total { margin: 0; font-variant-numeric: tabular-nums; }
  .pager { display: flex; align-items: center; justify-content: center; gap: 16px; margin-top: 32px; font-variant-numeric: tabular-nums; }
</style>
