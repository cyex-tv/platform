<script lang="ts">
  // A set's menu (right-click, the ⋯ button, Shift+F10 or the context-menu key on a set): Rename,
  // Copy link, Set active on <channel> (each channel of the owner you change), Change capacity,
  // Open the set page or its editor, Delete. What it offers is what the API lets you do
  // (set-menu.ts); until that is known only Copy link and Open show, with a quiet loading row.
  // Placed at the pointer (or under the button), kept 8px inside the window, in the top layer.
  // ↑ ↓ Home End move; Escape, Tab, a click outside, scrolling or resizing close it, and focus
  // goes back where it came from. The cyex menu pattern of browse/TileMenu.
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import { pop } from '$lib/motion';
  import type { MenuRights, MenuSet, SetAction } from './set-menu';
  import { platformName } from './set-menu';

  type Props = {
    set: MenuSet;
    x: number;
    y: number;
    rights: MenuRights | null;
    /** Signed in: rights are worth waiting for. */
    signedIn: boolean;
    /** Where the menu was opened: the set page leaves out "Open set page", the editor "Open in editor". */
    here?: 'page' | 'editor' | null;
    canRename: boolean;
    onaction: (a: SetAction) => void;
    onclose: (refocus: boolean) => void;
  };

  let { set, x, y, rights, signedIn, here = null, canRename, onaction, onclose }: Props = $props();

  let el = $state<HTMLDivElement>();
  let left = $state(0);
  let top = $state(0);
  const name = $derived(clean(set.name));
  const loading = $derived(signedIn && !rights);

  $effect(() => {
    void x;
    void y;
    void place();
  });
  // When the rights arrive the menu grows: keep it inside the window.
  $effect(() => {
    void rights;
    void keepInside();
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
    top = Math.round(y + el.offsetHeight + g > innerHeight ? Math.max(g, y - el.offsetHeight) : y);
    items()[0]?.focus({ preventScroll: true });
  }

  async function keepInside() {
    await tick();
    if (!el) return;
    const focusedInside = el.contains(document.activeElement);
    top = Math.round(Math.max(8, Math.min(top, innerHeight - 8 - el.offsetHeight)));
    if (!focusedInside) items()[0]?.focus({ preventScroll: true });
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
    else if (e.key === 'Escape' || e.key === 'Tab') {
      e.preventDefault();
      e.stopPropagation();
      onclose(true);
      return;
    }
    if (next >= 0) {
      e.preventDefault();
      list[next]?.focus();
    }
  }

  const act = (a: SetAction) => onaction(a);
</script>

<svelte:window onscroll={() => onclose(false)} onresize={() => onclose(false)} onblur={() => onclose(false)} />
<svelte:document
  onpointerdown={(e) => {
    if (el && !el.contains(e.target as Node)) onclose(false);
  }}
/>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="menu" role="menu" aria-label="Set {name}" popover="manual" bind:this={el} style:left="{left}px" style:top="{top}px"
  {onkeydown} oncontextmenu={(e) => e.preventDefault()} in:pop={{ duration: 120 }} data-testid="set-menu">
  {#if rights?.rename && canRename}
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => act({ kind: 'rename' })}><Icon name="pencil" size={14} /><span>Rename</span></button>
  {/if}
  <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => act({ kind: 'copy' })}><Icon name="link" size={14} /><span>Copy link</span></button>
  {#if rights?.channels.length}
    <hr />
    {#each rights.channels as c (c.platform + c.platformChannelId)}
      <button role="menuitem" class="item" type="button" tabindex="-1" aria-disabled={c.active}
        onclick={() => !c.active && act({ kind: 'active', platform: c.platform, platformChannelId: c.platformChannelId, login: c.login })}>
        <Icon name={c.active ? 'check' : c.platform === 'KICK' ? 'kick' : 'twitch'} size={14} />
        <span class="lbl">{c.active ? 'Active on' : 'Set active on'} {platformName(c.platform)} <bdi>{c.login}</bdi></span>
      </button>
    {/each}
  {/if}
  {#if rights?.capacity}
    <hr />
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => act({ kind: 'capacity' })}><Icon name="gauge" size={14} /><span>Change capacity…</span></button>
  {/if}
  <hr />
  {#if here !== 'page'}
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => act({ kind: 'page' })}><Icon name="arrow-right" size={14} /><span>Open set page</span></button>
  {/if}
  {#if rights?.edit && here !== 'editor'}
    <button role="menuitem" class="item" type="button" tabindex="-1" onclick={() => act({ kind: 'editor' })}><Icon name="settings" size={14} /><span>Open in editor</span></button>
  {/if}
  {#if loading}
    <span role="menuitem" class="item quiet" aria-disabled="true" tabindex="-1"><span class="spin" aria-hidden="true"></span><span>Checking what you can do…</span></span>
  {/if}
  {#if rights?.remove}
    <hr />
    <button role="menuitem" class="item danger" type="button" tabindex="-1" onclick={() => act({ kind: 'delete' })}><Icon name="trash" size={14} /><span>Delete…</span></button>
  {/if}
</div>

<style>
  .menu { position: fixed; inset: auto; margin: 0; z-index: var(--z-pop); min-width: 220px; max-width: min(320px, calc(100vw - 16px)); padding: 4px;
    border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--elevated); box-shadow: var(--shadow-pop);
    color: var(--text); overflow: visible; transform-origin: 0 0; }
  .item { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 32px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--text); font: inherit; font-size: var(--t-nav); text-align: left; text-decoration: none; cursor: pointer; }
  .item :global(svg) { flex: none; color: var(--muted); }
  .item > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item .lbl { flex: 1; }
  .item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .item:focus:not(:focus-visible) { outline: none; }
  .item[aria-disabled='true'] { color: var(--muted); cursor: default; }
  .item[aria-disabled='true'] :global(svg) { color: var(--accent-text); }
  .quiet { font-size: var(--t-small); }
  .spin { flex: none; width: 12px; height: 12px; border: 2px solid var(--border-strong); border-top-color: var(--muted); border-radius: 50%;
    animation: ui-spin .8s linear infinite; }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; } }
  .danger { color: var(--red); }
  .danger :global(svg) { color: var(--red); }
  hr { margin: 4px 0; border: 0; border-top: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) { .item:not([aria-disabled='true']):hover { background: var(--surface-hover); } }
  @media (pointer: coarse) { .item { min-height: 44px; } }
</style>
