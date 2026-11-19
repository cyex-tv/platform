<script lang="ts">
  // The one cyex menu for an emote tile (right-click, the ⋯ button, the context-menu key):
  // Add to / Remove from the target set, Open in new tab, Copy emote link, and Select (starts
  // multi-select with this emote). Placed at the pointer (or under the ⋯), kept 8px inside the
  // window, in the top layer so nothing clips it. Arrow keys, Home and End move; Escape, a click
  // outside, scrolling or Tab close it, and focus goes back to the tile.
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import { ApiError } from '$lib/api';
  import { copy } from '$lib/clipboard.svelte';
  import { clean } from '$lib/format';
  import { pop } from '$lib/motion';
  import { notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import Icon from '../Icon.svelte';
  import { target } from './target.svelte';

  type Props = {
    emote: { id: string; name: string };
    x: number;
    y: number;
    /** Multi-select is on (the Select item is left out then). */
    selecting: boolean;
    onselect: () => void;
    onclose: (refocus: boolean) => void;
  };
  let { emote, x, y, selecting, onselect, onclose }: Props = $props();

  let el = $state<HTMLDivElement>();
  let left = $state(0);
  let top = $state(0);
  let busy = $state(false);

  const set = $derived(target.current);
  const has = $derived(!!set && set.held.has(emote.id));
  const setName = $derived(set ? clean(set.name) : '');
  const link = $derived(`${page.url.origin}/emotes/${emote.id}`);
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`);

  $effect(() => {
    target.ensure();
  });

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

  const items = () => [...(el?.querySelectorAll<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"])') ?? [])];

  function onkeydown(e: KeyboardEvent) {
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLElement);
    let next = -1;
    if (e.key === 'ArrowDown') next = (at + 1) % list.length;
    else if (e.key === 'ArrowUp') next = (at - 1 + list.length) % list.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = list.length - 1;
    else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose(true);
      return;
    } else if (e.key === 'Tab') {
      e.preventDefault();
      onclose(true);
      return;
    }
    if (next >= 0) {
      e.preventDefault();
      list[next]?.focus();
    }
  }

  async function addOrRemove() {
    if (!set || busy) return;
    busy = true;
    try {
      if (has) {
        await target.remove(emote.id, set);
        notify(`Removed ${clean(emote.name)} from ${setName}.`);
      } else {
        await target.add(emote.id, set);
        notify(`Added ${clean(emote.name)} to ${setName}.`);
      }
      onclose(true);
    } catch (e) {
      const code = e instanceof ApiError ? e.code : '';
      notify(
        code === 'set_full'
          ? `${setName} is full.`
          : code === 'name_taken'
            ? `${setName} already has an emote called ${clean(emote.name)}.`
            : e instanceof Error
              ? e.message
              : 'That didn’t work.',
        true,
      );
      onclose(true);
    } finally {
      busy = false;
    }
  }

  async function copyLink() {
    const ok = await copy(link);
    notify(ok ? 'Emote link copied.' : 'Couldn’t copy the link.', !ok);
    onclose(true);
  }
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
  aria-label="{clean(emote.name)}"
  popover="manual"
  bind:this={el}
  style:left="{left}px"
  style:top="{top}px"
  {onkeydown}
  oncontextmenu={(e) => e.preventDefault()}
  in:pop={{ duration: 120 }}
>
  {#if !session.me}
    {#if session.known}
      <a role="menuitem" class="item" href={signin} tabindex="-1"><Icon name="plus" size={14} /><span>Sign in to add.</span></a>
    {/if}
  {:else if target.error}
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => target.refresh()}>
      <Icon name="plus" size={14} /><span>Couldn’t load your sets. Try again.</span>
    </button>
  {:else if !target.sets}
    <span role="menuitem" class="item" aria-disabled="true" tabindex="-1"><Icon name="plus" size={14} /><span>Loading your sets…</span></span>
  {:else if !set}
    <a role="menuitem" class="item" href="/me/sets" tabindex="-1"><Icon name="plus" size={14} /><span>Make a set to add to.</span></a>
  {:else}
    <button role="menuitem" class="item" class:danger={has} type="button" tabindex="-1" aria-busy={busy} onclick={addOrRemove}>
      <Icon name={has ? 'minus' : 'plus'} size={14} />
      <span class="lbl">{has ? 'Remove from' : 'Add to'} <bdi>{setName}</bdi></span>
    </button>
  {/if}
  <hr />
  <a role="menuitem" class="item" href="/emotes/{emote.id}" target="_blank" rel="noopener" tabindex="-1" onclick={() => onclose(false)}>
    <Icon name="arrow-right" size={14} /><span>Open in new tab</span>
  </a>
  <button role="menuitem" class="item" type="button" tabindex="-1" onclick={copyLink}><Icon name="link" size={14} /><span>Copy emote link</span></button>
  {#if !selecting}
    <hr />
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={onselect}><Icon name="check" size={14} /><span>Select</span></button>
  {/if}
</div>

<style>
  .menu { position: fixed; inset: auto; margin: 0; z-index: var(--z-pop); min-width: 200px; max-width: min(300px, calc(100vw - 16px)); padding: 4px;
    border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--elevated); box-shadow: var(--shadow-pop);
    color: var(--text); overflow: visible; transform-origin: 0 0; }
  .item { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 32px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--text); font: inherit; font-size: var(--t-nav); text-align: left; text-decoration: none; cursor: pointer; }
  .item :global(svg) { flex: none; color: var(--muted); }
  .item .lbl, .item > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .item:focus:not(:focus-visible) { outline: none; }
  .item[aria-disabled='true'] { color: var(--muted); cursor: default; }
  .item[aria-busy='true'] { opacity: .6; }
  .danger { color: var(--red); }
  .danger :global(svg) { color: var(--red); }
  hr { margin: 4px 0; border: 0; border-top: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) { .item:not([aria-disabled='true']):hover { background: var(--surface-hover); } }
  @media (pointer: coarse) { .item { min-height: 44px; } }
</style>
