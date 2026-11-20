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

<div class="docs changelog" bind:this={root}>
  <RailLayout>
    {#snippet rail()}
    <aside class="docs-nav" aria-label="Changelog index">
      <details class="nav-toc">
        <summary>Contents</summary>
        <nav class="nav-scroll" aria-label="Contents">
          {#each days as d (d.date)}
            <div class="nav-group">
              <div class="nav-group-title"><time class="group-name" datetime={d.date}>{shownDate(d.date)}</time></div>
              {#each d.entries as e (e.id)}
                <a class="nav-item" href="#{e.id}"><span class="nav-label">{e.title}</span></a>
              {/each}
            </div>
          {/each}
        </nav>
      </details>
    </aside>
    {/snippet}

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
  </RailLayout>
</div>

<style>
  .changelog :global(.nav-group-title) { display: flex; align-items: center; min-height: 32px; margin-bottom: 2px; font-variant-numeric: tabular-nums; }
  .changelog :global(.nav-item) { padding-block: 8px; }
  .changelog :global(.nav-label) { white-space: normal; }
  .entry { max-width: var(--measure); margin-bottom: 36px; }
  .entry + .entry { padding-top: 28px; border-top: 1px solid var(--border); }
  .when { margin: 0 0 4px; font-size: var(--t-fine); font-weight: 600; color: var(--dim); font-variant-numeric: tabular-nums; }
  .entry h2 { margin: 0 0 10px; }
  .entry ul { margin: 0; padding-left: 20px; display: grid; gap: 6px; }
  .entry li { color: var(--muted); line-height: 1.6; }
  .entry li::marker { color: var(--dim); }
  .staff { margin: 16px 0 6px; }
</style>
