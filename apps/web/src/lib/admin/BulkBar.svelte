<script lang="ts">
  // The sticky bar under an admin list while rows are picked: "n / 100 selected" (or what the last
  // bulk action did, or a short note such as the cap), the page's actions, and Clear. The same
  // look as the browse grid's selection bar.
  import type { Snippet } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { count } from '$lib/format';
  import { dur } from '$lib/motion';
  import { bottomBar } from '$lib/tool/bottom-bars.svelte';
  import { BATCH_MAX } from './bulk';

  type Props = {
    n: number;
    /** What is running ("Approving 12…"), or a note (the cap); shown in place of the count. */
    status?: string;
    busy?: boolean;
    /** "Select all on this page", when there is more to pick. */
    onall?: () => void;
    onclear: () => void;
    children: Snippet;
  };
  let { n, status = '', busy = false, onall, onclear, children }: Props = $props();
</script>

<div class="bar" role="region" aria-label="Selection" use:bottomBar in:fly|global={{ y: 8, duration: dur(200) }} out:fade|global={{ duration: dur(120) }}>
  <p class="n" aria-live="polite">
    {#if status}<span class="note">{status}</span>{:else}<b>{count(n)}</b> / {BATCH_MAX} selected{/if}
  </p>
  <div class="acts">
    {#if onall && !busy}<button class="link" type="button" onclick={onall}>Select all on this page</button>{/if}
    <button class="link" type="button" onclick={onclear} disabled={busy}>Clear</button>
    {@render children()}
  </div>
</div>

<style>
  .bar { position: sticky; bottom: 12px; z-index: 5; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between;
    gap: 8px 16px; margin-top: 16px; padding: 8px 8px 8px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card);
    background: var(--elevated-2); box-shadow: var(--shadow-pop); }
  .n { margin: 0; font-size: var(--t-nav); color: var(--muted); min-width: 0; font-variant-numeric: tabular-nums; }
  .n b { color: var(--text); font-weight: 700; }
  .note { color: var(--text); }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 8px; min-width: 0; }
  .link { min-height: 40px; padding: 0 4px; border: 0; background: none; color: var(--accent-text); font: inherit; font-size: var(--t-small); cursor: pointer; }
  .link:disabled { color: var(--dim); cursor: default; }
  @media (max-width: 639.98px) { .bar { bottom: 8px; padding-left: 12px; } .acts { width: 100%; justify-content: flex-end; } }
</style>
