<script lang="ts">
  import { media, PHONE } from '$lib/media.svelte';
  // A panel anchored to its trigger (filters, the account menu). It scales in from the corner
  // nearest the trigger and fades out a little faster (no motion under reduced motion), flips
  // above when there's no room below, and closes on Escape (focus back to the trigger, at once),
  // a click outside, or focus leaving it. With `sheet`, phones (720px and
  // narrower) get the same content in a BottomSheet instead.
  //
  // The panel lives in the browser's top layer (the popover API) and is placed with fixed
  // coordinates from the trigger's box, so no ancestor (the nav's glass, a sticky bar, a
  // transform, overflow) can clip it or paint over it. It follows the trigger while the page
  // scrolls and closes once the trigger has scrolled out of view.
  //
  // `kind="disclosure"` (the extension menu) is a plain show/hide region of links: the trigger
  // carries aria-expanded and aria-controls only, the panel has no dialog role, and focus stays
  // on the trigger (Tab walks straight into the links, which follow it in the DOM).
  //
  // `kind="menu"` (the account menu) is a real menu, like the emote tile's: opening it moves focus
  // to the first item, ↑ ↓ Home End move between the items (the content marks them
  // role="menuitem"), Escape closes it and returns focus to the trigger, and ↓ on the trigger opens it.
  //
  // `hover` (the account menu): a mouse pointer over the trigger opens it after a short delay (so
  // passing over doesn't flicker) and leaving trigger and panel closes it after a small grace;
  // clicks, keys and touch work as before. Opened by hover, it doesn't take focus, and a click on
  // the trigger then keeps it open instead of closing it.
  //
  // Whatever the kind, the panel keeps inside the window: it opens on the side with more room
  // and shrinks (its body scrolls) to the room there is.
  //
  //   <Popover label="Filters" sheet>
  //     {#snippet trigger(props)}<button class="btn btn-s" {...props}>Filters</button>{/snippet}
  //     …content…
  //   </Popover>
  import { tick, type Snippet } from 'svelte';
  import { createAttachmentKey } from 'svelte/attachments';
  import { pop } from '$lib/motion';
  import BottomSheet from './BottomSheet.svelte';

  type Props = {
    label: string;
    open?: boolean;
    align?: 'start' | 'end';
    width?: string;
    sheet?: boolean;
    hover?: boolean;
    /** Keep room for the scrollbar, so content doesn't shift when a long list starts to scroll. */
    stable?: boolean;
    kind?: 'dialog' | 'disclosure' | 'menu';
    onclose?: () => void;
    trigger: Snippet<[Record<string | symbol, unknown>]>;
    children: Snippet;
    footer?: Snippet;
  };
  let { label, open = $bindable(false), align = 'start', width = '320px', sheet = false, hover = false, stable = false, kind = 'dialog', onclose, trigger, children, footer }: Props =
    $props();

  const id = $props.id();
  let root: HTMLSpanElement;
  let panel = $state<HTMLDivElement>();
  let triggerEl: HTMLElement | null = null;
  let above = $state(false);
  let top = $state(0);
  let maxH = $state(560);
  let left = $state(0);
  const phone = $derived(media(PHONE).matches);

  const asSheet = $derived(sheet && phone);

  const HOVER_IN = 120;
  const HOVER_OUT = 260;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let byHover = false;

  function close(refocus = false) {
    clearTimeout(hoverTimer);
    if (!open) return;
    byHover = false;
    open = false;
    onclose?.();
    if (refocus) triggerEl?.focus();
  }

  const triggerProps = $derived({
    'aria-expanded': open,
    'aria-controls': open ? id : undefined, // only while the panel exists
    'aria-haspopup': kind === 'dialog' ? 'dialog' : kind === 'menu' ? 'menu' : undefined,
    onclick: () => {
      if (open && byHover) byHover = false; // hover opened it: a click keeps it
      else if (open) close();
      else open = true;
    },
    onkeydown: (e: KeyboardEvent) => {
      if (kind === 'menu' && !open && e.key === 'ArrowDown') {
        e.preventDefault();
        open = true;
      }
    },
    [createAttachmentKey()]: (el: HTMLElement) => {
      triggerEl = el;
    },
  });

  $effect(() => {
    if (!open || asSheet) return;
    void show();
    // Follow the trigger while anything scrolls or the window resizes.
    let frame = 0;
    const follow = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => place(false));
    };
    addEventListener('scroll', follow, { passive: true, capture: true });
    addEventListener('resize', follow, { passive: true });
    return () => {
      cancelAnimationFrame(frame);
      removeEventListener('scroll', follow, { capture: true });
      removeEventListener('resize', follow);
    };
  });

  const topLayer = typeof HTMLElement !== 'undefined' && 'showPopover' in HTMLElement.prototype;

  async function show() {
    await tick();
    if (!panel) return;
    if (topLayer) {
      try {
        panel.showPopover();
      } catch {}
    }
    place(true);
    if (kind === 'disclosure') return;
    if (byHover) return;
    if (kind === 'menu') {
      items()[0]?.focus({ preventScroll: true });
      return;
    }
    const first = panel.querySelector<HTMLElement>('a[href], button:not([disabled]), input, select, textarea, [tabindex="0"]');
    (first ?? panel).focus({ preventScroll: true });
  }

  /** Fixed coordinates from the trigger's box; `flip` re-decides above or below (only on open). */
  function place(flip: boolean) {
    if (!panel || !open) return;
    const t = (triggerEl ?? root).getBoundingClientRect();
    if (!flip && (t.bottom < 0 || t.top > innerHeight)) {
      close();
      return;
    }
    const w = panel.offsetWidth;
    // The height it wants (its body unscrolled), not the one it has now.
    const want = (panel.firstElementChild?.scrollHeight ?? 0) + (panel.children[1]?.clientHeight ?? 0) + 2;
    const edge = 8;
    const below = innerHeight - t.bottom - 6 - edge;
    const room = t.top - 6 - edge;
    if (flip) above = want > below && room > below;
    const cap = Math.max(120, Math.min(560, innerHeight * 0.7, above ? room : below));
    maxH = Math.floor(cap);
    const h = Math.min(want, cap);
    const gutter = 16;
    let x = align === 'end' ? t.right - w : t.left;
    x = Math.min(Math.max(x, gutter), innerWidth - gutter - w);
    left = Math.round(x);
    top = Math.round(Math.max(edge, Math.min(above ? t.top - 6 - h : t.bottom + 6, innerHeight - edge - h)));
  }

  const items = () => [...(panel?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])].filter((el) => el.offsetParent !== null);
  function menuKeys(e: KeyboardEvent) {
    const list = items();
    if (!list.length) return;
    const at = list.indexOf(document.activeElement as HTMLElement);
    let next = -1;
    if (e.key === 'ArrowDown') next = (at + 1) % list.length;
    else if (e.key === 'ArrowUp') next = (at - 1 + list.length) % list.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = list.length - 1;
    if (next < 0) return;
    e.preventDefault();
    list[next]?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (open && !asSheet && e.key === 'Escape') {
      e.stopPropagation();
      close(true);
    } else if (open && !asSheet && kind === 'menu' && panel?.contains(e.target as Node)) menuKeys(e);
  }
  function onpointerdown(e: PointerEvent) {
    if (open && !asSheet && !root.contains(e.target as Node)) close();
  }
  function onpointerenter(e: PointerEvent) {
    if (!hover || asSheet || e.pointerType !== 'mouse') return;
    clearTimeout(hoverTimer);
    if (open) return;
    hoverTimer = setTimeout(() => {
      byHover = true;
      open = true;
    }, HOVER_IN);
  }
  function onpointerleave(e: PointerEvent) {
    if (!hover || e.pointerType !== 'mouse') return;
    clearTimeout(hoverTimer);
    if (!open || !byHover) return;
    hoverTimer = setTimeout(() => {
      if (byHover) close();
    }, HOVER_OUT);
  }
  $effect(() => () => clearTimeout(hoverTimer));
  function onfocusout(e: FocusEvent) {
    const next = e.relatedTarget as Node | null;
    if (open && !asSheet && next && !root.contains(next)) close();
  }
</script>

<svelte:document {onpointerdown} />

<span class="pop-root" bind:this={root} {onkeydown} {onfocusout} {onpointerenter} {onpointerleave} role="presentation">
  {@render trigger(triggerProps)}
  {#if asSheet}
    <BottomSheet bind:open {label} onclose={() => onclose?.()} {footer}>{@render children()}</BottomSheet>
  {:else if open}
    <div
      class="pop {align}"
      class:above
      {id}
      role={kind === 'dialog' ? 'dialog' : kind === 'menu' ? 'menu' : undefined}
      aria-label={kind === 'disclosure' ? undefined : label}
      tabindex="-1"
      popover="manual"
      bind:this={panel}
      style:width
      style:max-height="{maxH}px"
      style:top="{top}px"
      style:left="{left}px"
      in:pop
      out:pop={{ duration: 100 }}
    >
      <div class="pop-body" class:stable>{@render children()}</div>
      {#if footer}<div class="pop-foot">{@render footer()}</div>{/if}
    </div>
  {/if}
</span>

<style>
  .pop-root { position: relative; display: inline-flex; }
  /* Top layer (popover="manual"): reset the UA popover box, place it with fixed coordinates. */
  .pop { position: fixed; inset: auto; margin: 0; padding: 0; overflow: visible; z-index: var(--z-pop);
    max-width: calc(100vw - 32px); max-height: min(70dvh, 560px); display: flex; flex-direction: column;
    border: 1px solid var(--border-strong); border-radius: var(--radius-card); background: var(--elevated);
    box-shadow: var(--shadow-pop); color: var(--text); text-align: left; }
  .pop:focus { outline: none; }
  .start { transform-origin: 0 0; }
  .end { transform-origin: 100% 0; }
  .start.above { transform-origin: 0 100%; }
  .end.above { transform-origin: 100% 100%; }
  .pop-body { padding: 8px; overflow-y: auto; overscroll-behavior: contain; }
  .pop-body.stable { scrollbar-gutter: stable; }
  .pop-foot { display: flex; justify-content: flex-end; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--border); }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) { .pop { border-color: var(--muted); } }
</style>
