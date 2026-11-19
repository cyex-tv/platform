<script lang="ts">
  // The emote page's Activity tab: who uploaded it and which sets added, renamed or removed it,
  // newest first, 30 to a page, each with its date and time. Only listed (public) emotes have
  // any, and the page doesn't mount this for the others. Pages are kept once loaded; the pager
  // sits in the tab bar, so this hands its state up (`pager`).
  import { onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { loadEmoteActivity, type ActivityItem } from '$lib/profile/activity';
  import EmoteActivityRows from './EmoteActivityRows.svelte';
  import { rangeLabel } from './emote-page';
  import { CursorTrail, type PagerState } from './pager';

  let { emoteId, name, pager = $bindable(null) }: { emoteId: string; name: string; pager?: PagerState | null } = $props();

  const pages = new SvelteMap<number, ActivityItem[]>();
  const trail = new CursorTrail();
  let at = $state(0);
  /** Bumped when the trail learns a page, so `next` is read again. */
  let known = $state(0);
  let busy = $state(false);
  let failed = $state<number | null>(null);

  const rows = $derived(pages.get(at) ?? []);
  const offset = $derived([...pages.entries()].filter(([i]) => i < at).reduce((n, [, r]) => n + r.length, 0));

  async function go(to: number) {
    if (busy || to < 0) return;
    failed = null;
    if (!pages.has(to)) {
      const start = trail.start(to);
      if (start === undefined) return;
      busy = true;
      try {
        const got = await loadEmoteActivity(emoteId, start);
        pages.set(to, got.items);
        trail.after(to, got.next);
        known++;
      } catch {
        failed = to;
        return;
      } finally {
        busy = false;
      }
    }
    at = to;
  }
  onMount(() => void go(0));

  $effect(() => {
    void known;
    pager = {
      range: rows.length && (at > 0 || trail.hasNext(at)) ? rangeLabel(offset, rows.length) : '',
      prev: at > 0,
      next: trail.hasNext(at),
      busy,
      onprev: () => go(at - 1),
      onnext: () => go(at + 1),
    };
  });
</script>

{#if failed !== null && !rows.length}
  <Problem cause="server" onretry={() => failed !== null && go(failed)} home={{ href: '/search', label: 'Browse emotes' }} />
{:else if !pages.has(at)}
  <State kind="loading" title="Loading activity…" />
{:else if !rows.length}
  <State title="No activity yet.">Sets adding, renaming or removing this emote show up here.</State>
{:else}
  <div class:busy><EmoteActivityRows items={rows} {name} /></div>
  {#if failed !== null}<p class="fine err" role="alert">That page didn’t load. <button class="link" type="button" onclick={() => failed !== null && go(failed)}>Try again</button></p>{/if}
{/if}

<style>
  p { margin: 0; }
  div { transition: opacity var(--dur-color); }
  .busy { opacity: .6; }
  .err { padding-top: 12px; color: var(--muted); }
  .err .link { padding: 0; border: 0; background: none; font: inherit; color: var(--text); text-decoration: underline; cursor: pointer; }
</style>
