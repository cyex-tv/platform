<script lang="ts">
  // /help: short answers to what people ask, on the cyex docs layout (cyex-docs) like /developers.
  // The index on the left lists every question by group and follows the reader (behaviour.ts); the
  // filter above it narrows the index and the answers together, matching every word typed against
  // the question and its answer. `/` focuses the filter (the palette leaves it to the page), and
  // ?q= opens the page filtered (the palette and other pages can link to a topic).
  import '$lib/docs/docs.css';
  import { onMount } from 'svelte';
  import { initDocs } from '$lib/docs/behaviour';
  import Prose from '$lib/docs/Prose.svelte';
  import State from '$lib/components/State.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import { registerShortcuts } from '$lib/shell/shortcuts.svelte';
  import { HELP_KEYS } from '$lib/docs/keys';
  import { apiUrl } from '$lib/api';
  import { checkStatus, HELP, matches, POPULAR, statusText } from '$lib/help/faq';

  let root: HTMLDivElement;
  let field: HTMLInputElement;
  let query = $state('');

  const shown = $derived(new Set(HELP.flatMap((g) => g.questions).filter((q) => matches(q, query)).map((q) => q.id)));
  const total = HELP.reduce((n, g) => n + g.questions.length, 0);
  const filtering = $derived(!!query.trim());

  // The service's own readiness check, asked once when the page opens.
  let status = $state<'checking' | 'ok' | 'down'>('checking');
  const statusUrl = `${apiUrl()}/health/ready`;

  onMount(() => {
    checkStatus(apiUrl()).then((ok) => (status = ok ? 'ok' : 'down'));
    const q = new URLSearchParams(location.search).get('q');
    if (q) query = q;
    return initDocs(root, { filter: false });
  });

  // The address keeps the filter, so a filtered page can be shared; no history entry per key.
  $effect(() => {
    const url = new URL(location.href);
    const q = query.trim();
    if ((url.searchParams.get('q') ?? '') === q) return;
    if (q) url.searchParams.set('q', q);
    else url.searchParams.delete('q');
    history.replaceState(history.state, '', url);
  });

  $effect(() => registerShortcuts(HELP_KEYS));

  function clear() {
    query = '';
    field.focus();
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && query) {
      e.preventDefault();
      query = '';
    }
  }
</script>

<svelte:head>
  <title>Help - cyex.tv</title>
  <meta name="description" content="Short answers about cyex.tv: the extension, uploading, sets and channels, editors, cosmetics, your account and takedowns." />
  <meta property="og:title" content="Help" />
</svelte:head>

{#snippet chev()}
  <svg viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 1.5 6.5 5 3 8.5" /></svg>
{/snippet}

<div class="docs help" bind:this={root}>
  <RailLayout>
    {#snippet rail()}
    <aside class="docs-nav" aria-label="Help index">
      <div class="nav-search" class:has-value={!!query}>
        <svg class="nav-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" /></svg>
        <input bind:this={field} bind:value={query} onkeydown={onkey} type="search" enterkeyhint="search" placeholder="Search help" aria-label="Search help"
          aria-controls="help-answers" autocomplete="off" autocapitalize="off" spellcheck="false" data-page-search />
        <button type="button" class="input-clear" aria-label="Clear" onclick={clear}><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 2l6 6M8 2L2 8" /></svg></button>
      </div>
      <details class="nav-toc">
        <summary>Contents</summary>
        <nav class="nav-scroll" aria-label="Contents">
          {#each HELP as g (g.id)}
            {@const n = g.questions.filter((q) => shown.has(q.id)).length}
            <div class="nav-group" hidden={!n}>
              <div class="nav-group-title"><span class="group-name" title={g.title}>{g.title}</span><span class="group-count" aria-hidden="true">{n}</span></div>
              {#each g.questions as q (q.id)}
                <a class="nav-item" href="#{q.id}" hidden={!shown.has(q.id)}><span class="nav-label">{q.q}</span></a>
              {/each}
            </div>
          {/each}
          {#if !shown.size}<p class="nav-empty">Nothing matches.</p>{/if}
        </nav>
      </details>
      <p class="sr-only" role="status">{filtering ? (shown.size ? `${shown.size} of ${total} answers` : 'No answer matches') : ''}</p>
    </aside>
    {/snippet}

    <div class="docs-split">
    <div class="docs-body" id="help-answers" tabindex="-1">
      <header class="docs-title">
        <div>
          <h1>Help</h1>
          <p>Short answers about cyex.tv. Not here? Ask in the <a href="https://github.com/cyex-tv/platform/issues">issue tracker</a>.</p>
        </div>
      </header>

      {#each HELP as g (g.id)}
        {@const visible = g.questions.filter((q) => shown.has(q.id))}
        <div class="help-group" hidden={!visible.length} aria-labelledby="g-{g.id}">
          <h2 class="op-group-title" id="g-{g.id}"><span>{g.title}</span><span class="count" aria-hidden="true">{visible.length}</span></h2>
          {#each g.questions as q (q.id)}
            <section class="guide" id={q.id} hidden={!shown.has(q.id)}>
              <h3>{q.q}</h3>
              <Prose paras={q.a} />
            </section>
          {/each}
        </div>
      {/each}

      {#if !shown.size}
        <State title="No answer matches “{query.trim()}”.">
          Try fewer words, or ask in the <a class="link" href="https://github.com/cyex-tv/platform/issues">issue tracker</a>.
          {#snippet actions()}<button type="button" class="btn btn-s" onclick={clear}>Clear search</button>{/snippet}
        </State>
      {/if}

      <p class="docs-end">Takedowns and legal requests: abuse@cyex.tv. Building on cyex.tv? See <a href="/developers">Developers</a>.</p>
    </div>

    <aside class="dp-panel" aria-label="Help panel">
      <section class="dp-sec" aria-labelledby="hp-status">
        <h2 class="dp-label" id="hp-status">Status</h2>
        <div class="dp-fact" class:ok={status === 'ok'} class:down={status === 'down'} role="status">
          <span class="dp-state"><span class="dp-dot" aria-hidden="true"></span>{statusText(status)}</span>
          <a href={statusUrl} rel="noopener">API status</a>
        </div>
      </section>
      <section class="dp-sec" aria-labelledby="hp-popular">
        <h2 class="dp-label" id="hp-popular">Popular</h2>
        <ul class="dp-rows">
          {#each POPULAR as q (q.id)}<li><a class="dp-row" href="#{q.id}"><span>{q.q}</span>{@render chev()}</a></li>{/each}
        </ul>
      </section>
      <section class="dp-sec" aria-labelledby="hp-contact">
        <h2 class="dp-label" id="hp-contact">Contact</h2>
        <ul class="dp-rows">
          <li><a class="dp-row" href="https://github.com/cyex-tv/platform/issues"><span>Issue tracker</span>{@render chev()}</a></li>
          <li><a class="dp-row" href="mailto:abuse@cyex.tv"><span>abuse@cyex.tv</span>{@render chev()}</a></li>
          <li><a class="dp-row" href="/developers"><span>Developers</span>{@render chev()}</a></li>
        </ul>
      </section>
    </aside>
    </div>
  </RailLayout>
</div>

<style>
  /* Wide windows: no scroll bar inside the index (owner: none in side lists). It runs the full
     height of the left column and the page scrolls; the search stays pinned at its top. */
  @media (min-width: 900px) {
    .help :global(.docs-nav) { position: relative; top: auto; align-self: stretch; height: auto; max-height: none; }
    .help :global(.nav-scroll) { overflow: visible; }
    .help :global(.nav-search) { position: sticky; top: calc(var(--docs-top) + 12px); z-index: 1; margin-right: 0;
      padding-right: 12px; background: var(--bg); box-shadow: 0 -12px 0 var(--bg); }
  }
  .help :global(.nav-group-title) { display: flex; align-items: center; min-height: 32px; margin-bottom: 2px; }
  /* Questions are sentences: they wrap in the index instead of being cut. */
  .help :global(.nav-item) { padding-block: 8px; }
  .help :global(.nav-label) { white-space: normal; }
  .help :global(.help-group[hidden]), .help :global(section.guide[hidden]) { display: none; }
  .help :global(.help-group) { max-width: var(--measure); }
  .help :global(.help-group + .help-group) { margin-top: 28px; }
  /* A group's title: uppercase label, mono count at the right, hairline below. */
  .help :global(.help-group .op-group-title) { display: flex; align-items: center; justify-content: space-between; min-height: 40px; margin: 0; padding: 0; border-bottom: 1px solid var(--border); }
  .help :global(.help-group .count) { font-family: var(--font-mono); font-size: 12px; font-weight: 500; letter-spacing: 0; font-variant-numeric: tabular-nums; }
  /* Each question is a block between hairlines. */
  .help :global(section.guide) { margin: 0; padding: 20px 0; border-bottom: 1px solid var(--border); }
  .help :global(section.guide h3) { margin: 0 0 6px; font-size: 1.05rem; font-weight: 600; line-height: 1.35; color: var(--text); }
  .help :global(section.guide p:last-child) { margin-bottom: 0; }
</style>
