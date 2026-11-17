<script lang="ts">
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
    onclose?: () => void;
    trigger: Snippet<[Record<string | symbol, unknown>]>;
    children: Snippet;
    footer?: Snippet;
  };
  let { label, open = $bindable(false), align = 'start', width = '320px', sheet = false, onclose, trigger, children, footer }: Props =
    $props();

  const id = $props.id();
  let root: HTMLSpanElement;
  let panel = $state<HTMLDivElement>();
  let triggerEl: HTMLElement | null = null;
  let above = $state(false);
  let top = $state(0);
  let left = $state(0);
  let phone = $state(false);

  $effect(() => {
    if (!sheet) return;
    const mq = matchMedia('(max-width: 720px)');
    phone = mq.matches;
    const on = () => (phone = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  const asSheet = $derived(sheet && phone);

  function close(refocus = false) {
    if (!open) return;
    open = false;
    onclose?.();
    if (refocus) triggerEl?.focus();
  }

  const triggerProps = $derived({
    'aria-expanded': open,
    'aria-controls': open ? id : undefined, // only while the panel exists
    'aria-haspopup': 'dialog',
    onclick: () => (open ? close() : (open = true)),
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
    const h = panel.offsetHeight;
    if (flip) above = t.bottom + h + 12 > innerHeight && t.top - h - 12 > 0;
    const gutter = 16;
    let x = align === 'end' ? t.right - w : t.left;
    x = Math.min(Math.max(x, gutter), innerWidth - gutter - w);
    left = Math.round(x);
    top = Math.round(above ? t.top - 6 - h : t.bottom + 6);
  }

  function onkeydown(e: KeyboardEvent) {
    if (open && !asSheet && e.key === 'Escape') {
      e.stopPropagation();
      close(true);
    }
  }
  function onpointerdown(e: PointerEvent) {
    if (open && !asSheet && !root.contains(e.target as Node)) close();
  }
  function onfocusout(e: FocusEvent) {
    const next = e.relatedTarget as Node | null;
    if (open && !asSheet && next && !root.contains(next)) close();
  }
</script>

<svelte:document {onpointerdown} />

<span class="pop-root" bind:this={root} {onkeydown} {onfocusout} role="presentation">
  {@render trigger(triggerProps)}
  {#if asSheet}
    <BottomSheet bind:open {label} onclose={() => onclose?.()} {footer}>{@render children()}</BottomSheet>
  {:else if open}
    <div
      class="pop {align}"
      class:above
      {id}
      role="dialog"
      aria-label={label}
      tabindex="-1"
      popover="manual"
      bind:this={panel}
      style:width
      style:top="{top}px"
      style:left="{left}px"
      in:pop
      out:pop={{ duration: 100 }}
    >
      <div class="pop-body">{@render children()}</div>
      {#if footer}<div class="pop-foot">{@render footer()}</div>{/if}
    </div>
  {/if}
</span>

<style>
  .pop-root { position: relative; display: inline-flex; }
  /* Top layer (popover="manual"): reset the UA popover box, place it with fixed coordinates. */
  .pop { position: fixed; inset: auto; margin: 0; padding: 0; overflow: visible; z-index: var(--z-pop);
    max-width: calc(100vw - 32px); max-height: min(70dvh, 560px); display: flex; flex-direction: column;
    border: 1px solid var(--border-strong); border-radius: var(--radius-m); background: var(--surface);
    box-shadow: var(--shadow-pop); color: var(--text); text-align: left; }
  .pop:focus { outline: none; }
  .start { transform-origin: 0 0; }
  .end { transform-origin: 100% 0; }
  .start.above { transform-origin: 0 100%; }
  .end.above { transform-origin: 100% 100%; }
  .pop-body { padding: 8px; overflow-y: auto; overscroll-behavior: contain; }
  .pop-foot { display: flex; justify-content: flex-end; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--border); }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) { .pop { border-color: var(--muted); } }
</style>
