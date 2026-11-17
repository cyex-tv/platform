<script lang="ts">
  // The foot of a list that loads in pages: the button (or a plain link when `href` is given, so
  // it works without scripts), an error with a retry, or the one closing line. The two button
  // labels share one grid cell, so "Loading…" never changes the width.
  import { count } from '$lib/format';

  type Props = { hasMore: boolean; busy?: boolean; error?: string; total: number; onmore: () => void; href?: string };
  let { hasMore, busy = false, error = '', total, onmore, href }: Props = $props();

  function click(e: MouseEvent) {
    e.preventDefault();
    if (!busy) onmore();
  }
</script>

<div class="more">
  {#if error}
    <p class="muted fine" role="alert">{error}</p>
    <button class="btn btn-s" type="button" onclick={onmore}>Try again</button>
  {:else if hasMore}
    {#if href}
      <a class="btn btn-s go" {href} onclick={click} aria-disabled={busy}>
        <span class="lab" class:off={busy}>Load more</span><span class="lab" class:off={!busy} aria-hidden={!busy}>Loading…</span>
      </a>
    {:else}
      <button class="btn btn-s go" type="button" onclick={onmore} disabled={busy} aria-busy={busy}>
        <span class="lab" class:off={busy}>Load more</span><span class="lab" class:off={!busy} aria-hidden={!busy}>Loading…</span>
      </button>
    {/if}
  {:else if total > 0}
    <p class="muted fine">That’s all {count(total)}.</p>
  {/if}
</div>

<style>
  .more { display: grid; justify-items: center; gap: 12px; }
  .more p { margin: 0; }
  .go { display: inline-grid; min-width: 12ch; place-items: center; }
  .lab { grid-area: 1 / 1; }
  .off { visibility: hidden; }
  .go[aria-disabled='true'] { pointer-events: none; opacity: .7; }
</style>
