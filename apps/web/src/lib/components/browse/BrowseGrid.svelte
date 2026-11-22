<script lang="ts">
  // The browse page's one large grid (EmoteTile, the same tile as everywhere) with the ways to
  // pick and act on emotes:
  //   - right-click, the context-menu key, the ⋯ button (keyboard focus, hover, always on touch):
  //     the cyex tile menu; the browser's own menu never shows on a tile.
  //   - Select mode: click or Ctrl/⌘+click toggles one tile and makes it the anchor; Shift+click
  //     gives the range from the anchor to the clicked tile the state the clicked tile ends up
  //     with (on an unpicked tile it picks the range, on a picked one it unpicks it). Space
  //     toggles the focused tile (Shift+Space a range, the same way).
  //   - Mouse or pen: press and drag draws a rectangle, file-manager style: started on a picked
  //     tile it unpicks every tile it touches, otherwise it picks them; with Ctrl/⌘ it flips
  //     each one. From an empty spot it also turns Select mode on. It scrolls the window near
  //     its edges; Esc mid-drag puts the picks back as they were before it.
  //   - Touch: long-press turns Select mode on with that tile; in Select mode it picks or (on a
  //     picked tile) unpicks it. Dragging on without lifting goes on the same way.
  //   - Signed in, a + on every tile adds it to the target set in place (Undo in the toast; a
  //     full set is refused); a tile already in the target says so and its button removes it.
  //   - Drag a tile onto a set in the rail to add it there (outside Select mode; in Select mode a
  //     press and drag is the rectangle, and the picks go to a set from the bar or the menu).
  //   - Right-click a picked tile in Select mode: the menu acts on every picked emote.
  // Everything goes through the pure model in ./selection, which keeps the cap.
  // Tiles load their emote's page on tap, not on hover (each page is four queries, and a drag
  // across the grid would fetch dozens), and not at all while Select mode is on.
  import { isWide } from '$lib/emote';
  import { clean } from '$lib/format';
  import EmoteSkeleton from '../EmoteSkeleton.svelte';
  import EmoteTile, { type CardEmote } from '../EmoteTile.svelte';
  import Icon from '../Icon.svelte';
  import TileMenu from './TileMenu.svelte';
  import * as S from './selection';
  import { SvelteSet } from 'svelte/reactivity';
  import { session } from '$lib/tool/session.svelte';
  import { endDrag, startDrag } from './drag';
  // Not imported as `target`: the test runner's module rewrite would also rename Svelte's `new.target`.
  import { target as store, setLabel, type SetItem } from './target.svelte';

  type Props = {
    items: CardEmote[];
    label: string;
    loading?: boolean;
    skeletons?: number;
    eager?: number;
    ranked?: boolean;
    selecting: boolean;
    sel: S.Selection;
    /** A selection change (and how many the cap left out). */
    onstep: (step: S.Step) => void;
    /** Turn Select mode on, picking `id` when given. */
    onstart: (id?: string) => void;
    /** Turn Select mode off again: Esc on a drag that turned it on. */
    onstop?: () => void;
    /** A contact sheet (redesign round 2): the tiles share hairlines, no gaps and no cards. */
    sheet?: boolean;
  };
  let { items, label, loading = false, skeletons = 10, eager = 0, ranked = false, selecting, sel, onstep, onstart, onstop, sheet = false }: Props = $props();
  // The set Add goes to: emotes already in it get an "In set" mark (the tile menu says Remove for them).
  const held = $derived.by(() => {
    const t = store.current;
    return t ? { set: t, name: clean(t.name), label: setLabel(t), has: (id: string) => t.held.has(id) } : null;
  });
  // Signed in with a set to add to: every tile gets its + (outside Select mode).
  const adding = $derived(!!session.me && !session.ended && !!held);
  const asItem = (e: CardEmote): SetItem => ({ id: e.id, name: e.shownName ?? e.name, host: e.host, animated: e.animated, files: e.files });
  const busy = new SvelteSet<string>();
  async function addOrRemove(e: CardEmote) {
    const h = held;
    if (!h || busy.has(e.id)) return;
    busy.add(e.id);
    try {
      if (h.has(e.id)) await store.take(h.set, [e.id]);
      else await store.put(h.set, [asItem(e)]);
    } finally {
      busy.delete(e.id);
    }
  }

  let root = $state<HTMLDivElement>();

  // A long grid is a long Tab walk: a "Skip results" link ahead of the tiles (shown when it has
  // focus, like the page's skip link) moves focus to a marker just after them.
  const endId = $props.id();
  const long = $derived(items.length > 8);
  function skip(e: MouseEvent) {
    e.preventDefault();
    const end = document.getElementById(endId);
    end?.focus();
    end?.scrollIntoView({ block: 'center' });
  }
  const order = $derived(items.map((e) => e.id));
  const picked = $derived(new Set(sel.ids));

  const liOf = (t: EventTarget | null) => (t instanceof Element ? t.closest<HTMLElement>('li[data-id]') : null);
  const onMore = (t: EventTarget | null) => t instanceof Element && !!t.closest('.more');

  // ---- The tile menu.
  let menu = $state<{ emotes: CardEmote[]; id: string; x: number; y: number } | null>(null);
  function openMenu(id: string, x: number, y: number) {
    const emote = items.find((e) => e.id === id);
    if (!emote) return;
    // A picked tile in Select mode: the menu is for every picked emote.
    const many = selecting && picked.has(id) && sel.ids.length > 1;
    const emotes = many ? (sel.ids.map((p) => items.find((e) => e.id === p)).filter(Boolean) as CardEmote[]) : [emote];
    menu = { emotes, id, x, y };
  }
  function closeMenu(refocus: boolean) {
    const id = menu?.id;
    menu = null;
    if (refocus && id) root?.querySelector<HTMLElement>(`li[data-id="${CSS.escape(id)}"] .tile`)?.focus();
  }

  // ---- Pointer state. `swallow` eats the click that ends a drag or a long-press.
  let lastPointer = '';
  let swallow = false;
  let band = $state<S.Box | null>(null);
  type Press = {
    start: S.Point;
    client: S.Point;
    dragging: boolean;
    /** The picks when the press began: every move recomputes from them, and Esc puts them back. */
    base: S.Selection;
    mode: S.DragMode;
    /** This drag turned Select mode on (so Esc turns it off again). */
    started: boolean;
    boxes: (S.Box & { id: string })[];
  };
  let press: Press | null = null;
  let frame = 0;
  let longPress: ReturnType<typeof setTimeout> | undefined;

  const pagePoint = (c: S.Point): S.Point => ({ x: c.x + scrollX, y: c.y + scrollY });

  function measure() {
    return [...(root?.querySelectorAll<HTMLElement>('li[data-id]') ?? [])].map((li) => {
      const r = li.getBoundingClientRect();
      return { id: li.dataset.id!, left: r.left + scrollX, top: r.top + scrollY, right: r.right + scrollX, bottom: r.bottom + scrollY };
    });
  }

  function update() {
    if (!press?.dragging) return;
    if (press.boxes.length !== order.length) press.boxes = measure(); // more pages came in
    const r = S.rect(press.start, pagePoint(press.client));
    band = r;
    onstep(S.drag(press.base, S.hitTest(r, press.boxes), press.mode));
  }

  function loop() {
    if (!press?.dragging) return;
    const v = S.edgeSpeed(press.client.y, innerHeight);
    if (v) scrollBy({ top: v, behavior: 'instant' });
    update();
    frame = requestAnimationFrame(loop);
  }

  const touchOpts: AddEventListenerOptions = { passive: false };
  function listen(on: boolean) {
    if (on) {
      addEventListener('pointermove', onpointermove);
      addEventListener('pointerup', onpointerup);
      addEventListener('pointercancel', onpointerup);
      addEventListener('keydown', onescape, true);
      // A touch drag (after a long-press) must not scroll the page: only a non-passive touchmove can say so.
      addEventListener('touchmove', notouchscroll, touchOpts);
    } else {
      removeEventListener('pointermove', onpointermove);
      removeEventListener('pointerup', onpointerup);
      removeEventListener('pointercancel', onpointerup);
      removeEventListener('keydown', onescape, true);
      removeEventListener('touchmove', notouchscroll, touchOpts);
    }
  }

  function onpointerdown(ev: PointerEvent) {
    lastPointer = ev.pointerType;
    swallow = false;
    if (ev.button !== 0 || onMore(ev.target)) return;
    const li = liOf(ev.target);
    if (ev.pointerType === 'touch') {
      if (li) startLongPress(li, ev);
      return;
    }
    // Outside Select mode a press on a tile is the link's; from an empty spot it may become a drag.
    if (!selecting && li) return;
    const client = { x: ev.clientX, y: ev.clientY };
    const base = selecting ? sel : S.EMPTY;
    const mode = S.dragMode(base, li?.dataset.id ?? null, ev.ctrlKey || ev.metaKey);
    press = { start: pagePoint(client), client, dragging: false, base, mode, started: false, boxes: [] };
    listen(true);
  }

  function onpointermove(ev: PointerEvent) {
    if (!press) return;
    press.client = { x: ev.clientX, y: ev.clientY };
    if (!press.dragging) {
      if (!S.moved(press.start, pagePoint(press.client))) return;
      press.dragging = true;
      press.boxes = measure();
      if (!selecting) {
        onstart();
        press.started = true;
      }
      getSelection()?.removeAllRanges();
      frame = requestAnimationFrame(loop);
    }
    update();
  }

  function onpointerup() {
    listen(false);
    cancelAnimationFrame(frame);
    // A finger's click comes a little after it lifts; a mouse's straight after.
    if (press?.dragging) {
      swallow = true;
      setTimeout(() => (swallow = false), lastPointer === 'touch' ? 400 : 0);
    }
    press = null;
    band = null;
  }

  // Esc mid-drag: the picks go back to what they were before it (and Select mode off, if the drag
  // turned it on). Caught first, so the page's own Esc (clear, then leave) doesn't run as well.
  function onescape(ev: KeyboardEvent) {
    if (ev.key !== 'Escape' || !press?.dragging) return;
    ev.preventDefault();
    ev.stopPropagation();
    const { base, started } = press;
    onpointerup();
    // The button is still down: the click its release makes must not toggle a tile either
    // (the next press clears this).
    clearTimeout(longPress);
    swallow = true;
    onstep(S.cancelDrag(base));
    if (started) onstop?.();
  }

  function notouchscroll(ev: TouchEvent) {
    if (press?.dragging && ev.cancelable) ev.preventDefault();
  }

  function startLongPress(li: HTMLElement, ev: PointerEvent) {
    clearTimeout(longPress);
    const from = { x: ev.clientX, y: ev.clientY };
    const id = li.dataset.id!;
    const cancel = () => {
      clearTimeout(longPress);
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', cancel);
      removeEventListener('pointercancel', cancel);
    };
    const move = (m: PointerEvent) => {
      if (S.moved(from, { x: m.clientX, y: m.clientY }, 10)) cancel();
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', cancel);
    addEventListener('pointercancel', cancel);
    longPress = setTimeout(() => {
      cancel();
      swallow = true;
      setTimeout(() => (swallow = false), 600);
      navigator.vibrate?.(8);
      // Outside Select mode it starts it with this tile picked. In it, a long-press on a picked
      // tile unpicks it and starts an unpicking drag, on an unpicked one a picking drag.
      const base = selecting ? sel : S.EMPTY;
      const mode = S.dragMode(base, id);
      const started = !selecting;
      if (started) onstart(id);
      const start = pagePoint(from);
      press = { start, client: from, dragging: true, base, mode, started, boxes: measure() };
      listen(true);
      onstep(S.drag(base, [id], mode));
      frame = requestAnimationFrame(loop);
    }, 480);
  }

  // ---- Clicks and keys, caught before the tile (a link) sees them.
  function onclickcapture(ev: MouseEvent) {
    if (swallow) {
      ev.preventDefault();
      ev.stopPropagation();
      swallow = false;
      return;
    }
    if (!selecting || onMore(ev.target)) return;
    const li = liOf(ev.target);
    if (!li) return;
    ev.preventDefault();
    ev.stopPropagation();
    const id = li.dataset.id!;
    onstep(ev.shiftKey ? S.range(sel, order, id) : S.toggle(sel, id));
  }

  function onkeydowncapture(ev: KeyboardEvent) {
    lastPointer = ''; // the context-menu key opens the menu even after a touch
    if (ev.key !== ' ' || !selecting || onMore(ev.target)) return;
    const li = liOf(ev.target);
    if (!li) return;
    ev.preventDefault();
    ev.stopPropagation();
    const id = li.dataset.id!;
    onstep(ev.shiftKey ? S.range(sel, order, id) : S.toggle(sel, id));
  }

  function oncontextmenu(ev: MouseEvent) {
    const li = liOf(ev.target);
    if (!li) return;
    ev.preventDefault();
    // A long-press on touch is Select (handled above), not the menu.
    if (lastPointer === 'touch') return;
    // From the keyboard (context-menu key, Shift+F10): under the tile's top-left corner.
    if (ev.clientX === 0 && ev.clientY === 0) {
      const r = li.getBoundingClientRect();
      openMenu(li.dataset.id!, r.left + 8, r.top + 32);
    } else openMenu(li.dataset.id!, ev.clientX, ev.clientY);
  }

  function moreClick(ev: MouseEvent, id: string) {
    const r = (ev.currentTarget as HTMLElement).getBoundingClientRect();
    openMenu(id, r.left, r.bottom + 4);
  }

  $effect(() => () => {
    onpointerup();
    clearTimeout(longPress);
  });

  // ---- Dragging onto the rail: one tile, or the picked ones. A count stands in for the picture.
  function dragstart(ev: DragEvent) {
    const li = liOf(ev.target);
    const dt = ev.dataTransfer;
    if (!li || !dt || !session.me) {
      if (selecting) ev.preventDefault();
      return;
    }
    // In Select mode a press and drag is the rectangle (above), never a drag onto the rail.
    if (selecting) {
      ev.preventDefault();
      return;
    }
    const id = li.dataset.id!;
    const list = items.filter((e) => e.id === id);
    startDrag(dt, list.map(asItem));
    if (list.length > 1) {
      const ghost = document.createElement('div');
      ghost.className = 'drag-ghost';
      ghost.textContent = `${list.length} emotes`;
      document.body.append(ghost);
      dt.setDragImage(ghost, 12, 12);
      requestAnimationFrame(() => ghost.remove());
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="pick-area"
  class:selecting
  bind:this={root}
  data-sveltekit-preload-data={selecting ? 'off' : 'tap'}
  {onpointerdown}
  onclickcapture={onclickcapture}
  onkeydowncapture={onkeydowncapture}
  {oncontextmenu}
  ondragstart={dragstart}
  ondragend={endDrag}
>
  {#if long}<a class="skip" href="#{endId}" onclick={skip}>Skip results</a>{/if}
  <ul class="grid" class:contact-sheet={sheet} class:sheet aria-label={label} aria-busy={loading}>
    {#each items as e, i (e.id)}
      <li class:wide={isWide(e)} data-id={e.id}>
        <EmoteTile
          emote={e}
          eager={i < eager}
          onpick={selecting ? () => onstep(S.toggle(sel, e.id)) : undefined}
          selected={picked.has(e.id)}
          rank={ranked ? i + 1 : undefined}
          inSet={held?.has(e.id) ? held.name : undefined}
        />
        {#if adding && !selecting && held}
          {@const on = held.has(e.id)}
          <button
            class="add"
            class:on
            type="button"
            aria-pressed={on}
            aria-busy={busy.has(e.id)}
            aria-label="{on ? 'Remove' : 'Add'} {clean(e.name)} {on ? 'from' : 'to'} {held.label}"
            title="{on ? 'Remove from' : 'Add to'} {held.label}"
            onclick={() => addOrRemove(e)}
          >
            <Icon name={on ? 'check' : 'plus'} size={16} />
          </button>
        {/if}
        {#if !selecting}
          <button class="more" class:low={adding} type="button" aria-label="More for {clean(e.name)}" aria-haspopup="menu" onclick={(ev) => moreClick(ev, e.id)}>
            <Icon name="more" size={16} />
          </button>
        {/if}
      </li>
    {/each}
    {#if loading}
      {#each Array(skeletons) as _, i (i)}
        <li class="sk"><EmoteSkeleton density="comfortable" creator /></li>
      {/each}
    {/if}
  </ul>
  {#if long}<span class="end" id={endId} tabindex="-1"></span>{/if}
  {#if band && root}
    {@const o = root.getBoundingClientRect()}
    <div
      class="band"
      aria-hidden="true"
      style:left="{band.left - (o.left + scrollX)}px"
      style:top="{band.top - (o.top + scrollY)}px"
      style:width="{band.right - band.left}px"
      style:height="{band.bottom - band.top}px"
    ></div>
  {/if}
</div>

{#if menu}
  <TileMenu
    emotes={menu.emotes}
    x={menu.x}
    y={menu.y}
    {selecting}
    onclose={closeMenu}
    onselect={() => {
      const id = menu?.id;
      closeMenu(true);
      onstart(id);
    }}
  />
{/if}

<style>
  .pick-area { position: relative; }
  .end { position: absolute; width: 0; height: 0; outline: none; }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 16px;
    grid-template-columns: repeat(auto-fill, minmax(var(--emote-tile), 1fr)); grid-auto-flow: row dense; }
  .wide { grid-column: span 2; }
  li { position: relative; min-width: 0; }
  /* Long-press is ours on touch: no link callout, no text selection. */
  .pick-area :global(.tile) { -webkit-touch-callout: none; }
  .selecting { user-select: none; -webkit-user-select: none; }
  .selecting :global(.tile) { cursor: default; }

  /* ⋯: quiet, in the tile's top-right corner; shows on hover and keyboard focus, always on touch. */
  .more { position: absolute; top: 6px; right: 6px; z-index: 1; display: inline-grid; place-items: center; width: 28px; height: 28px;
    padding: 0; border: 1px solid transparent; border-radius: var(--radius); background: var(--surface); color: var(--muted);
    cursor: pointer; opacity: 0; transition: opacity var(--fast) ease-out, color var(--fast); }
  li:has(:focus-visible) .more { opacity: 1; }
  .more:focus-visible { opacity: 1; outline-offset: 0; }
  @media (hover: hover) and (pointer: fine) {
    li:hover .more { opacity: 1; }
    .more:hover { color: var(--text); border-color: var(--border-strong); }
  }
  @media (hover: none) { .more { opacity: 1; background: none; } }
  /* Touch: 28px to see, 44px to hit (8px of invisible reach on every side). */
  @media (pointer: coarse) { .more::after { content: ''; position: absolute; inset: -8px; } }

  /* +: the tile's own add button, top-right, always there (signed in). In the target: a tick on
     the accent hairline, and the click removes it. */
  .add { position: absolute; top: 6px; right: 6px; z-index: 1; display: inline-grid; place-items: center; width: 30px; height: 30px; padding: 0;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--bg); color: var(--text); cursor: pointer;
    transition: color var(--fast), border-color var(--fast), background-color var(--fast), transform var(--dur-press) ease-out; touch-action: manipulation; }
  .add:active { transform: scale(var(--press)); }
  .add.on { border-color: var(--accent-border); color: var(--accent-text); }
  .add[aria-busy='true'] { opacity: .6; pointer-events: none; }
  .add:focus-visible { outline-offset: 1px; }
  .more.low { top: 42px; }
  @media (hover: hover) and (pointer: fine) {
    .add:hover { border-color: var(--accent-border); color: var(--accent-text); }
    .add.on:hover { border-color: var(--red); color: var(--red); }
  }
  @media (pointer: coarse) { .add::after { content: ''; position: absolute; inset: -7px; } }
  :global(.drag-ghost) { position: fixed; top: -100px; left: 0; padding: 6px 10px; border-radius: var(--radius-btn); background: var(--accent);
    color: var(--accent-ink); font: 600 13px/1.2 var(--font, inherit); }

  /* The drag rectangle: a hairline in the accent over a faint tint of it. */
  .band { position: absolute; z-index: 2; pointer-events: none; border: 1px solid var(--accent-border); border-radius: 2px;
    background: color-mix(in srgb, var(--accent) 8%, transparent); }

  .grid.sheet { gap: 0; }
  @media (max-width: 520px) { .grid { gap: 8px; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); } .grid.sheet { gap: 0; } }
  @media (forced-colors: active) { .band { border-color: Highlight; } }
</style>
