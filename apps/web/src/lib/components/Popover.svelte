<script lang="ts">
  // A panel anchored to its trigger (filters, the account menu). It scales in from the corner
  // nearest the trigger, flips above when there's no room below, and closes on Escape (focus
  // back to the trigger), a click outside, or focus leaving it. With `sheet`, phones (720px and
  // narrower) get the same content in a BottomSheet instead.
  //
  //   <Popover label="Filters" sheet>
  //     {#snippet trigger(props)}<button class="btn btn-s" {...props}>Filters</button>{/snippet}
  //     …content…
  //   </Popover>
  import { tick, type Snippet } from 'svelte';
  import { createAttachmentKey } from 'svelte/attachments';
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

  const id = `pop-${Math.random().toString(36).slice(2, 9)}`;
  let root: HTMLSpanElement;
  let panel = $state<HTMLDivElement>();
  let triggerEl: HTMLElement | null = null;
  let above = $state(false);
  let shift = $state(0);
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
    'aria-controls': id,
    'aria-haspopup': 'dialog',
    onclick: () => (open ? close() : (open = true)),
    [createAttachmentKey()]: (el: HTMLElement) => {
      triggerEl = el;
    },
  });

  $effect(() => {
    if (!open || asSheet) return;
    void place();
  });

  async function place() {
    await tick();
    if (!panel) return;
    const t = root.getBoundingClientRect();
    const p = panel.getBoundingClientRect();
    above = t.bottom + p.height + 12 > innerHeight && t.top - p.height - 12 > 0;
    // Keep it inside the viewport sideways (16px gutter).
    const left = p.left - shift;
    const right = left + p.width;
    shift = right > innerWidth - 16 ? innerWidth - 16 - right : left < 16 ? 16 - left : 0;
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    panel.animate(
      still ? [{ opacity: 0 }, { opacity: 1 }] : [{ opacity: 0, transform: 'scale(.97)' }, { opacity: 1, transform: 'scale(1)' }],
      { duration: 150, easing: 'cubic-bezier(.23, 1, .32, 1)' },
    );
    const first = panel.querySelector<HTMLElement>('a[href], button:not([disabled]), input, select, textarea, [tabindex="0"]');
    (first ?? panel).focus({ preventScroll: true });
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
      bind:this={panel}
      style:width
      style:translate="{shift}px 0"
    >
      <div class="pop-body">{@render children()}</div>
      {#if footer}<div class="pop-foot">{@render footer()}</div>{/if}
    </div>
  {/if}
</span>

<style>
  .pop-root { position: relative; display: inline-flex; }
  .pop { position: absolute; top: calc(100% + 6px); z-index: var(--z-pop); max-width: calc(100vw - 32px);
    max-height: min(70dvh, 560px); display: flex; flex-direction: column;
    border: 1px solid var(--border-strong); border-radius: var(--radius-m); background: var(--surface);
    box-shadow: var(--shadow-pop); color: var(--text); text-align: left; }
  .pop:focus { outline: none; }
  .start { left: 0; transform-origin: 0 0; }
  .end { right: 0; transform-origin: 100% 0; }
  .above { top: auto; bottom: calc(100% + 6px); }
  .start.above { transform-origin: 0 100%; }
  .end.above { transform-origin: 100% 100%; }
  .pop-body { padding: 8px; overflow-y: auto; overscroll-behavior: contain; }
  .pop-foot { display: flex; justify-content: flex-end; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--border); }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) { .pop { border-color: var(--muted); } }
</style>
