<script lang="ts">
  // The browse page's tray (1280px and wider, signed in): what the target set holds, live. Its
  // name, how full it is (meter, free slots, LIVE when a channel shows it), then its emotes newest
  // first in fixed-height rows, each with a remove button (Undo in the toast). A set holds up to a
  // thousand, so the list shows a hundred at a time. The tray folds to a slim strip (`open`,
  // remembered by the page) and opens again from there.
  import { reduced } from '$lib/cosmetics/reduced.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import { clean, count } from '$lib/format';
  import { target as store, setLabel, free } from './target.svelte';

  type Props = { open: boolean; ontoggle: () => void };
  let { open, ontoggle }: Props = $props();

  const STEP = 100;
  const set = $derived(store.current);
  let steps = $state(1);
  $effect(() => {
    void set?.id;
    steps = 1;
  });
  const items = $derived(set ? [...set.items.values()].reverse() : []);
  const shown = $derived(items.slice(0, steps * STEP));
  const pct = $derived(set && set.capacity > 0 ? Math.min(100, (set.held.size / set.capacity) * 100) : 0);
  // Under reduced motion animated emotes show their first frame, as everywhere.
  const still = (e: { animated: boolean; files: { name: string }[] }) => reduced.on && e.animated && e.files.some((f) => f.name === '1x_static.webp');
</script>

{#if !open}
  <div class="strip">
    <IconButton icon="chevron-left" label={set ? `Show ${setLabel(set)}` : 'Show the set'} tip="none" onclick={ontoggle} aria-expanded="false" />
    {#if set}<span class="vt" aria-hidden="true">{clean(set.name)} · {count(set.held.size)}</span>{/if}
  </div>
{:else}
  <div class="tray">
    <div class="head">
      <h2 class="t"><span class="sr-only">In </span><bdi>{set ? setLabel(set) : 'Your set'}</bdi></h2>
      {#if set}<span class="ct">{count(set.held.size)}/{count(set.capacity)}</span>{/if}
      <IconButton icon="chevron-right" label="Hide the set" size="s" tip="none" onclick={ontoggle} aria-expanded="true" />
    </div>
    {#if !set}
      <p class="msg">{store.sets ? 'Pick a set on the left to see what it holds.' : 'Loading your set…'}</p>
    {:else}
      <div class="meter-row">
        <span class="meter" class:full={free(set) === 0} aria-hidden="true"><i style:width="{pct}%"></i></span>
        <span class="free">{free(set) === 0 ? 'Full' : `${count(free(set))} free ${free(set) === 1 ? 'slot' : 'slots'}`}{#if set.active}{' · '}<span class="live">live in chat</span>{/if}</span>
      </div>
      {#if items.length === 0}
        <p class="msg">Nothing in {clean(set.name)} yet. Use <b>+</b> on an emote to add it here.</p>
      {:else}
        <ul class="rows" aria-label="Emotes in {clean(set.name)}">
          {#each shown as e (e.id)}
            <li class="row">
              <span class="pic"><img src="{e.host}/{still(e) ? '1x_static' : '1x'}.webp" alt="" loading="lazy" height="28" /></span>
              <a class="nm" href="/emotes/{encodeURIComponent(e.id)}"><bdi>{clean(e.name)}</bdi></a>
              <IconButton icon="minus" label="Remove {clean(e.name)} from {clean(set.name)}" size="s" tip="none" onclick={() => store.take(set, [e.id])} />
            </li>
          {/each}
        </ul>
        {#if shown.length < items.length}
          <div class="more">
            <span>{count(shown.length)} of {count(items.length)}</span>
            <button class="btn btn-s" type="button" onclick={() => steps++}>Show {count(Math.min(STEP, items.length - shown.length))} more</button>
          </div>
        {/if}
      {/if}
    {/if}
  </div>
{/if}

<style>
  p, h2 { margin: 0; }
  .tray { display: grid; align-content: start; min-width: 0; }
  .head { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 6px 0 14px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .t { flex: 1; min-width: 0; font-size: var(--t-small); font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ct { flex: none; font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .meter-row { display: grid; gap: 6px; padding: 12px 14px; border-bottom: 1px solid var(--border); }
  .meter { height: 4px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); transition: width var(--base, .2s) var(--ease); }
  .meter.full i { background: var(--red); }
  .free { font-size: var(--t-fine); color: var(--muted); }
  .live { color: var(--green); font-weight: 600; }
  .msg { padding: 14px; font-size: var(--t-small); color: var(--muted); }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: 10px; height: 44px; padding: 0 6px 0 14px; border-bottom: 1px solid var(--border); }
  .pic { flex: none; display: grid; place-items: center; width: 40px; height: 28px; }
  .pic img { display: block; max-width: 40px; max-height: 28px; width: auto; height: auto; }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 10px 14px; font-size: var(--t-fine); color: var(--muted); }
  .strip { display: grid; justify-items: center; gap: 10px; padding: 6px 0; }
  .vt { writing-mode: vertical-rl; font-size: var(--t-fine); color: var(--muted); font-weight: 600; white-space: nowrap; max-height: 60vh; overflow: hidden; text-overflow: ellipsis; }
  @media (hover: hover) and (pointer: fine) { .nm:hover { text-decoration: underline; text-underline-offset: 3px; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .meter i { transition: none; } }
</style>
