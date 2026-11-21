<script lang="ts">
  // /changelog: what changed on cyex.tv, newest first, on the cyex docs layout like /help. The
  // index lists the days and their entries and follows the reader (behaviour.ts); the entries are
  // data ($lib/changelog/entries). Opening the page marks the newest entry seen on this device,
  // which clears the "What's new" dot in the account menu.
  import '$lib/docs/docs.css';
  import { onMount } from 'svelte';
  import { initDocs } from '$lib/docs/behaviour';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import { byDay, ENTRIES, shownDate } from '$lib/changelog/entries';
  import { whatsNew } from '$lib/changelog/seen.svelte';

  let root: HTMLDivElement;
  const days = byDay(ENTRIES);

  onMount(() => {
    whatsNew.markSeen();
    return initDocs(root, { filter: false });
  });
</script>

<svelte:head>
  <title>Changelog - cyex.tv</title>
  <meta name="description" content="What changed on cyex.tv, newest first: new features and fixes for the site, the extension and the API." />
  <meta property="og:title" content="Changelog" />
</svelte:head>

{#snippet chev()}
  <svg viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 1.5 6.5 5 3 8.5" /></svg>
{/snippet}

<div class="docs changelog" bind:this={root}>
  <RailLayout>
    {#snippet rail()}
    <aside class="docs-nav" aria-label="Changelog index">
      <details class="nav-toc">
        <summary>Contents</summary>
        <nav class="nav-scroll" aria-label="Contents">
          {#each days as d (d.date)}
            <div class="nav-group">
              <div class="nav-group-title"><time class="group-name" datetime={d.date}>{shownDate(d.date)}</time><span class="count" aria-hidden="true">{d.entries.length}</span></div>
              {#each d.entries as e (e.id)}
                <a class="nav-item" href="#{e.id}"><span class="nav-label">{e.title}</span></a>
              {/each}
            </div>
          {/each}
        </nav>
      </details>
    </aside>
    {/snippet}

    <div class="docs-split">
    <div class="docs-body" id="changes" tabindex="-1">
      <header class="docs-title">
        <div>
          <h1>Changelog</h1>
          <p>What changed on cyex.tv, newest first. Ideas or problems? Tell us in the <a href="https://github.com/cyex-tv/platform/issues">issue tracker</a>.</p>
        </div>
      </header>

      {#each ENTRIES as e (e.id)}
        <section class="guide entry" id={e.id} aria-labelledby="{e.id}-h">
          <p class="when"><time datetime={e.date}>{shownDate(e.date)}</time></p>
          <h2 id="{e.id}-h">{e.title}</h2>
          <ul>{#each e.items as item (item)}<li>{item}</li>{/each}</ul>
          {#if e.staff?.length}
            <h3 class="ui-label staff">For staff</h3>
            <ul>{#each e.staff as item (item)}<li>{item}</li>{/each}</ul>
          {/if}
        </section>
      {/each}

      <p class="docs-end">Older changes are in the <a href="https://github.com/cyex-tv/platform/pulls?q=is%3Apr+is%3Amerged">merged pull requests</a>.</p>
    </div>

    <aside class="dp-panel" aria-label="Changelog panel">
      <section class="dp-sec" aria-labelledby="cp-numbers">
        <h2 class="dp-label" id="cp-numbers">In numbers</h2>
        <dl class="dp-facts">
          <div class="dp-fact"><dt>Entries</dt><dd class="mono">{ENTRIES.length}</dd></div>
          <div class="dp-fact"><dt>Days</dt><dd class="mono">{days.length}</dd></div>
          <div class="dp-fact"><dt>Latest</dt><dd class="mono"><time datetime={ENTRIES[0].date}>{shownDate(ENTRIES[0].date)}</time></dd></div>
        </dl>
      </section>
      <section class="dp-sec" aria-labelledby="cp-more">
        <h2 class="dp-label" id="cp-more">More</h2>
        <ul class="dp-rows">
          <li><a class="dp-row" href="https://github.com/cyex-tv/platform/issues"><span>Issue tracker</span>{@render chev()}</a></li>
          <li><a class="dp-row" href="https://github.com/cyex-tv/platform/pulls?q=is%3Apr+is%3Amerged"><span>Merged pull requests</span>{@render chev()}</a></li>
        </ul>
      </section>
    </aside>
    </div>
  </RailLayout>
</div>

<style>
  .changelog :global(.nav-group-title) { font-variant-numeric: tabular-nums; }
  .changelog :global(.nav-item) { padding-block: 8px; }
  .changelog :global(.nav-label) { white-space: normal; }
  .changelog :global(.nav-group-title time) { font-family: var(--font-mono); letter-spacing: 0; }
  /* One row per entry: the date in a fixed mono column, the title and its bullets beside it. */
  .changelog .entry { display: grid; grid-template-columns: 120px minmax(0, 1fr); gap: 0 20px; max-width: var(--measure); margin: 0; padding: 22px 0; border-bottom: 1px solid var(--border); }
  .changelog .entry:first-of-type { border-top: 1px solid var(--border); }
  .changelog .when { grid-row: 1 / span 3; margin: 3px 0 0; font-family: var(--font-mono); font-size: var(--t-fine); font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }
  .changelog .entry > :not(.when) { grid-column: 2; min-width: 0; }
  .changelog .entry h2 { margin: 0 0 10px; font-size: 1.1rem; }
  .changelog .entry ul { margin: 0; padding-left: 20px; display: grid; gap: 6px; }
  .changelog .entry li { color: var(--muted); line-height: 1.6; }
  .changelog .entry li::marker { color: var(--dim); }
  .changelog .staff { margin: 16px 0 6px; }
  @media (max-width: 620px) {
    .changelog .entry { grid-template-columns: minmax(0, 1fr); }
    .changelog .when { grid-row: auto; margin: 0 0 4px; }
    .changelog .entry > :not(.when) { grid-column: 1; }
  }
</style>
