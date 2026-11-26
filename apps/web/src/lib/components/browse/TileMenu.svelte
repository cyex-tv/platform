<script lang="ts">
  // The one cyex menu for emote tiles (right-click, the ⋯ button, the context-menu key), for one
  // emote or for the picked ones (right-click a picked tile in Select mode):
  //   Add to / Remove from the target set; "Add to…", a second page listing every other set you
  //   can edit (grouped like the rail, a filter past 8 sets, each with how full it is); Remove
  //   from each other set that holds the emote; Open, Open in new tab, Copy link; Favourite, Hide
  //   for me and Hide everything by the creator (signed in, one emote); Select.
  // Placed at the pointer (or under the ⋯), kept 8px inside the window, in the top layer so
  // nothing clips it. ↑ ↓ Home End move; → or Enter opens "Add to…", ← or Escape comes back from
  // it; Escape, a click outside, scrolling or Tab close the menu, and focus goes back to the tile.
  import { tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { copy } from '$lib/clipboard.svelte';
  import { clean, count } from '$lib/format';
  import { pop } from '$lib/motion';
  import { notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { listError, personal } from '$lib/personal/lists.svelte';
  import Icon from '../Icon.svelte';
  import type { CardEmote } from '../EmoteTile.svelte';
  import { FILTER_FROM, filterGroups, groupSets } from './set-filter';
  import { target as store, setLabel, type SetItem, type TargetSet } from './target.svelte';

  type Props = {
    /** The emote right-clicked, or every picked one. */
    emotes: CardEmote[];
    x: number;
    y: number;
    /** Multi-select is on (the Select item is left out then). */
    selecting: boolean;
    onselect: () => void;
    onclose: (refocus: boolean) => void;
  };
  let { emotes, x, y, selecting, onselect, onclose }: Props = $props();

  let el = $state<HTMLDivElement>();
  let left = $state(0);
  let top = $state(0);
  let view = $state<'main' | 'sets'>('main');
  let query = $state('');

  const one = $derived(emotes.length === 1 ? emotes[0] : null);
  const ids = $derived(emotes.map((e) => e.id));
  const set = $derived(store.current);
  const inTarget = $derived(set ? ids.filter((id) => set.held.has(id)) : []);
  const notInTarget = $derived(set ? ids.filter((id) => !set.held.has(id)) : ids);
  const others = $derived((store.sets ?? []).filter((s) => s.id !== set?.id));
  /** The other sets holding the emote (one emote only): each gets a Remove row. */
  const holders = $derived(one ? others.filter((s) => s.held.has(one.id)) : []);
  const groups = $derived(groupSets(others));
  const filterable = $derived(others.length > FILTER_FROM);
  const shownGroups = $derived(filterGroups(groups, filterable ? query : ''));
  const label = $derived(one ? clean(one.name) : `${count(emotes.length)} emotes`);
  const link = (id: string) => `${page.url.origin}/emotes/${id}`;
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`);

  $effect(() => {
    store.ensure();
    personal.ensure();
  });

  const fav = $derived(!!one && personal.favourites.has(one.id));
  const creatorId = $derived(one?.owner?.id ?? '');
  const creatorName = $derived(clean(one?.owner?.displayName ?? ''));
  const mine = $derived(!!creatorId && creatorId === session.me?.id);
  const asItems = (list: CardEmote[]): SetItem[] => list.map((e) => ({ id: e.id, name: e.shownName ?? e.name, host: e.host, animated: e.animated, files: e.files }));

  // The grid drops this menu (and its props) on close, so every value is read before that.
  function putIn(s: TargetSet, which: CardEmote[]) {
    const items = asItems(which);
    onclose(true);
    void store.put(s, items);
  }
  function takeOut(s: TargetSet, which: string[]) {
    onclose(true);
    void store.take(s, which);
  }

  async function toggleFavourite() {
    if (!one) return;
    const id = one.id;
    const name = clean(one.name);
    const was = fav;
    onclose(true);
    try {
      if (was) {
        await personal.unfavourite(id);
        notify(`Removed ${name} from your favourites.`);
      } else {
        await personal.favourite(id);
        notify(`Added ${name} to your favourites.`);
      }
    } catch (e) {
      notify(listError(e), true);
    }
  }

  async function hide(creator: boolean) {
    if (!one) return;
    const id = one.id;
    const name = clean(one.name);
    const who = creatorId;
    const whoName = creatorName;
    onclose(false);
    const undo = (run: () => Promise<unknown>) => ({ label: 'Undo', run: () => void run().catch((e) => notify(listError(e), true)) });
    try {
      if (creator) {
        await personal.hideCreator(who);
        notify(`Hid everything by ${whoName} for you.`, false, { ms: 6000, action: undo(() => personal.unhideCreator(who)) });
      } else {
        await personal.hideEmote(id);
        notify(`Hid ${name} for you.`, false, { ms: 6000, action: undo(() => personal.unhideEmote(id)) });
      }
    } catch (e) {
      notify(listError(e), true);
    }
  }

  async function copyLinks() {
    const text = ids.map(link).join('\n');
    onclose(true);
    const ok = await copy(text);
    notify(ok ? (one ? 'Emote link copied.' : `${count(ids.length)} links copied.`) : 'Couldn’t copy the link.', !ok);
  }

  function open() {
    if (!one) return;
    const href = `/emotes/${one.id}`;
    onclose(false);
    void goto(href);
  }

  $effect(() => {
    void x;
    void y;
    void place();
  });

  async function place() {
    await tick();
    if (!el) return;
    try {
      el.showPopover();
    } catch {
      /* no top layer: fixed position still works */
    }
    const g = 8;
    left = Math.round(Math.min(Math.max(x, g), innerWidth - g - el.offsetWidth));
    // Below the point when it fits, else above it.
    top = Math.round(y + el.offsetHeight + g > innerHeight ? Math.max(g, y - el.offsetHeight) : y);
    items()[0]?.focus({ preventScroll: true });
  }

  async function show(next: 'main' | 'sets') {
    view = next;
    query = '';
    await tick();
    // Keep it inside the window after the page changed its height.
    if (el) top = Math.round(Math.min(top, innerHeight - 8 - el.offsetHeight));
    if (next === 'sets') (el?.querySelector<HTMLElement>('input') ?? items()[1] ?? items()[0])?.focus({ preventScroll: true });
    else el?.querySelector<HTMLElement>('[data-sub]')?.focus({ preventScroll: true });
  }

  const items = () => [...(el?.querySelectorAll<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"]), input') ?? [])];

  function onkeydown(e: KeyboardEvent) {
    const list = items();
    const active = document.activeElement as HTMLElement;
    const at = list.indexOf(active);
    let next = -1;
    if (e.key === 'ArrowDown') next = (at + 1) % list.length;
    else if (e.key === 'ArrowUp') next = (at - 1 + list.length) % list.length;
    else if (e.key === 'Home' && active.tagName !== 'INPUT') next = 0;
    else if (e.key === 'End' && active.tagName !== 'INPUT') next = list.length - 1;
    else if (e.key === 'ArrowRight' && active.hasAttribute('data-sub')) {
      e.preventDefault();
      void show('sets');
      return;
    } else if ((e.key === 'ArrowLeft' && active.tagName !== 'INPUT') || (e.key === 'Escape' && view === 'sets')) {
      if (view === 'sets') {
        e.preventDefault();
        e.stopPropagation();
        if (e.key === 'Escape' && query) query = '';
        else void show('main');
      }
      return;
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose(true);
      return;
    } else if (e.key === 'Tab') {
      e.preventDefault();
      onclose(true);
      return;
    } else if (e.key === 'Enter' && active.tagName === 'INPUT') {
      const first = shownGroups[0]?.sets[0];
      if (first) {
        e.preventDefault();
        putIn(first, emotes);
      }
      return;
    }
    if (next >= 0) {
      e.preventDefault();
      list[next]?.focus();
    }
  }

  const pct = (s: TargetSet) => (s.capacity > 0 ? Math.min(100, (s.held.size / s.capacity) * 100) : 0);
</script>

<svelte:window onscroll={() => onclose(false)} onresize={() => onclose(false)} onblur={() => onclose(false)} />
<svelte:document
  onpointerdown={(e) => {
    if (el && !el.contains(e.target as Node)) onclose(false);
  }}
/>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  class="menu"
  role="menu"
  aria-label={view === 'sets' ? `Add ${label} to` : label}
  popover="manual"
  bind:this={el}
  style:left="{left}px"
  style:top="{top}px"
  {onkeydown}
  oncontextmenu={(e) => e.preventDefault()}
  in:pop={{ duration: 120 }}
>
  {#if view === 'sets'}
    <button role="menuitem" class="item back" type="button" tabindex="-1" onclick={() => show('main')}>
      <Icon name="chevron-left" size={14} /><span>Add {label} to…</span>
    </button>
    {#if filterable}
      <div class="find">
        <Icon name="search" size={13} />
        <input type="search" placeholder="Set, account or channel" aria-label="Find a set" autocomplete="off" spellcheck="false" bind:value={query} />
      </div>
    {/if}
    <div class="sets">
      {#each shownGroups as g (g.key)}
        <p class="grp" role="presentation"><bdi>{g.label}</bdi>{#if g.key !== 'mine'}<span>{' · editor'}</span>{/if}</p>
        {#each g.sets as s (s.id)}
          {@const all = ids.every((id) => s.held.has(id))}
          <button role="menuitem" class="item set" type="button" tabindex="-1" aria-disabled={all} onclick={() => !all && putIn(s, emotes)}>
            <Icon name={all ? 'check' : 'folder'} size={14} />
            <span class="lbl"><bdi>{clean(s.name)}</bdi>{#if s.active}<span class="live">Live</span>{/if}</span>
            <span class="fill" aria-hidden="true"><span class="meter" class:full={s.held.size >= s.capacity}><i style:width="{pct(s)}%"></i></span>{count(s.held.size)}/{count(s.capacity)}</span>
            <span class="sr-only">, {count(s.held.size)} of {count(s.capacity)} used{all ? ', already holds it' : ''}</span>
          </button>
        {/each}
      {:else}
        <p class="none">{others.length ? 'No set matches.' : 'You have no other sets.'}</p>
      {/each}
    </div>
  {:else}
    {#if !session.me}
      {#if session.known}
        <a role="menuitem" class="item" href={signin} tabindex="-1"><Icon name="plus" size={14} /><span>Sign in to add.</span></a>
      {/if}
    {:else if store.error}
      <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => store.refresh()}>
        <Icon name="plus" size={14} /><span>Couldn’t load your sets. Try again.</span>
      </button>
    {:else if !store.sets}
      <span role="menuitem" class="item" aria-disabled="true" tabindex="-1"><Icon name="plus" size={14} /><span>Loading your sets…</span></span>
    {:else if !set}
      <a role="menuitem" class="item" href="/me/sets" tabindex="-1"><Icon name="plus" size={14} /><span>Make a set to add to.</span></a>
    {:else}
      {#if notInTarget.length}
        <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => putIn(set, emotes.filter((e) => notInTarget.includes(e.id)))}>
          <Icon name="plus" size={14} /><span class="lbl">Add {one ? '' : `${count(notInTarget.length)} `}to <bdi>{setLabel(set)}</bdi></span>
        </button>
      {/if}
      {#if inTarget.length}
        <button role="menuitem" class="item danger" type="button" tabindex="-1" onclick={() => takeOut(set, inTarget)}>
          <Icon name="minus" size={14} /><span class="lbl">Remove {one ? '' : `${count(inTarget.length)} `}from <bdi>{setLabel(set)}</bdi></span>
        </button>
      {/if}
      {#if others.length}
        <button role="menuitem" class="item" type="button" tabindex="-1" aria-haspopup="menu" data-sub onclick={() => show('sets')}>
          <Icon name="folder" size={14} /><span class="lbl">Add to…</span><Icon name="chevron-right" size={14} />
        </button>
      {/if}
      {#each holders as s (s.id)}
        <button role="menuitem" class="item danger" type="button" tabindex="-1" onclick={() => takeOut(s, ids)}>
          <Icon name="minus" size={14} /><span class="lbl">Remove from <bdi>{setLabel(s)}</bdi></span>
        </button>
      {/each}
    {/if}
    <hr />
    {#if one}
      <button role="menuitem" class="item" type="button" tabindex="-1" onclick={open}><Icon name="arrow-right" size={14} /><span>Open</span></button>
      <a role="menuitem" class="item" href="/emotes/{one.id}" target="_blank" rel="noopener" tabindex="-1" onclick={() => onclose(false)}>
        <Icon name="open" size={14} /><span>Open in new tab</span>
      </a>
    {/if}
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={copyLinks}><Icon name="link" size={14} /><span>{one ? 'Copy link' : 'Copy links'}</span></button>
    {#if session.me && one}
      <button role="menuitem" class="item" class:fav type="button" tabindex="-1" onclick={toggleFavourite}>
        <Icon name="heart" size={14} /><span>{fav ? 'Remove from favourites' : 'Favourite'}</span>
      </button>
      <hr />
      <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => hide(false)}><Icon name="eye-off" size={14} /><span>Hide for me</span></button>
      {#if creatorId && !mine}
        <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => hide(true)}>
          <Icon name="user-x" size={14} /><span class="lbl">Hide everything by <bdi>{creatorName}</bdi></span>
        </button>
      {/if}
    {/if}
    {#if !selecting}
      <hr />
      <button role="menuitem" class="item" type="button" tabindex="-1" onclick={onselect}><Icon name="check" size={14} /><span>Select</span></button>
    {/if}
  {/if}
</div>

<style>
  p { margin: 0; }
  .menu { position: fixed; inset: auto; margin: 0; z-index: var(--z-pop); min-width: 220px; max-width: min(320px, calc(100vw - 16px)); padding: 4px;
    border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--elevated); box-shadow: var(--shadow-pop);
    color: var(--text); overflow: visible; transform-origin: 0 0; }
  .item { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 32px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--text); font: inherit; font-size: var(--t-nav); text-align: left; text-decoration: none; cursor: pointer; }
  .item :global(svg) { flex: none; color: var(--muted); }
  .item .lbl, .item > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item .lbl { flex: 1; }
  .item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .item:focus:not(:focus-visible) { outline: none; }
  .item[aria-disabled='true'] { color: var(--muted); cursor: default; }
  .danger { color: var(--red); }
  .danger :global(svg) { color: var(--red); }
  .fav :global(svg) { color: var(--accent-text); fill: currentColor; }
  .back { color: var(--muted); font-weight: 600; }
  hr { margin: 4px 0; border: 0; border-top: 1px solid var(--border); }

  .find { position: relative; display: flex; align-items: center; margin: 4px 0; }
  .find :global(svg) { position: absolute; left: 9px; color: var(--dim); pointer-events: none; }
  .find input { width: 100%; height: 32px; padding: 0 8px 0 28px; border: 1px solid var(--border-strong); border-radius: var(--radius);
    background: var(--bg); color: var(--text); font: inherit; font-size: var(--t-small); }
  .find input::-webkit-search-cancel-button { display: none; }
  .find input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 0; }
  .sets { max-height: min(360px, 60vh); overflow-y: auto; overscroll-behavior: contain; }
  .grp { padding: 8px 8px 4px; font-size: var(--t-fine); font-weight: 600; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grp span { font-weight: 400; color: var(--dim); }
  .set .lbl { display: flex; align-items: center; gap: 6px; }
  .set .lbl bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .live { flex: none; font-size: 10px; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--green); }
  .fill { flex: none; display: inline-flex; align-items: center; gap: 6px; font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .meter { width: 36px; height: 3px; border-radius: var(--radius-sm); background: var(--border); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); }
  .meter.full i { background: var(--red); }
  .none { padding: 8px; font-size: var(--t-small); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .item:not([aria-disabled='true']):hover { background: var(--surface-hover); } }
  @media (pointer: coarse) { .item { min-height: 44px; } }
</style>
