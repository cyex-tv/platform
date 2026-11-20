<script lang="ts">
  // A sheet that rises from the bottom, for phones (filters, the nav menu). A modal <dialog>:
  // focus stays inside, the page behind is inert, Escape and a tap on the dimmed page close it,
  // and focus goes back to what opened it. The handle can be dragged down to dismiss (a quick
  // flick is enough). Under reduced motion it fades instead of sliding.
  import type { Snippet } from 'svelte';
  import IconButton from './IconButton.svelte';

  type Props = {
    open: boolean;
    label: string;
    /** Show `label` as a visible title (default) or only to assistive tech. */
    title?: boolean;
    onclose?: () => void;
    children: Snippet;
    footer?: Snippet;
  };
  let { open = $bindable(), label, title = true, onclose, children, footer }: Props = $props();

  let dialog: HTMLDialogElement;
  let panel: HTMLDivElement;
  let back: HTMLElement | null = null;
  let closing = false;

  const still = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const ms = (name: string) => parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name)) * 1000 || 250;
  const ease = () => getComputedStyle(document.documentElement).getPropertyValue('--ease-drawer').trim() || 'ease-out';

  function show() {
    back = document.activeElement as HTMLElement | null;
    dialog.showModal();
    // Reduced motion (the OS's or the site's): it simply appears.
    if (still()) return;
    const d = ms('--dur-sheet');
    panel.animate([{ transform: 'translateY(100%)' }, { transform: 'translateY(0)' }], { duration: d, easing: ease() });
    try {
      dialog.animate([{ opacity: 0 }, { opacity: 1 }], { duration: d, easing: 'ease-out', pseudoElement: '::backdrop' });
    } catch {}
  }

  async function hide(from = 0) {
    if (closing || !dialog.open) return;
    closing = true;
    if (!still()) {
      const d = Math.round(ms('--dur-sheet') * 0.75);
      try {
        dialog.animate([{ opacity: 1 }, { opacity: 0 }], { duration: d, easing: 'ease-out', pseudoElement: '::backdrop', fill: 'forwards' });
      } catch {}
      await panel.animate([{ transform: `translateY(${from}px)` }, { transform: 'translateY(100%)' }], { duration: d, easing: 'ease-out', fill: 'forwards' }).finished.catch(() => {});
    }
    dialog.close();
    for (const a of [...panel.getAnimations(), ...dialog.getAnimations({ subtree: false })]) a.cancel();
    panel.style.transform = '';
    closing = false;
    back?.focus?.();
    back = null;
  }

  $effect(() => {
    if (open && !dialog.open) show();
    else if (!open && dialog.open) hide();
  });

  function close() {
    open = false;
    onclose?.();
  }

  function oncancel(e: Event) {
    // Escape: animate out instead of vanishing.
    e.preventDefault();
    close();
  }

  function onclick(e: MouseEvent) {
    // The panel fills the dialog, so a click that lands on the dialog itself was on the backdrop.
    if (e.target === dialog) close();
  }

  // Drag to dismiss: follows the finger 1:1 downwards, resists upwards.
  let drag: { y: number; t: number; dy: number; v: number } | null = null;
  function down(e: PointerEvent) {
    if (e.button !== 0 || drag || still()) return;
    if ((e.target as HTMLElement).closest('button, a, input')) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { y: e.clientY, t: e.timeStamp, dy: 0, v: 0 };
    for (const a of panel.getAnimations()) a.cancel();
  }
  function moveDrag(e: PointerEvent) {
    if (!drag) return;
    const raw = e.clientY - drag.y;
    const dt = Math.max(1, e.timeStamp - drag.t);
    drag.v = (raw - drag.dy) / dt;
    drag.t = e.timeStamp;
    drag.dy = raw;
    const y = raw >= 0 ? raw : -Math.sqrt(-raw) * 2; // rubber band past the top
    panel.style.transform = `translateY(${y}px)`;
  }
  function up() {
    if (!drag) return;
    const { dy, v } = drag;
    drag = null;
    if (dy > panel.offsetHeight * 0.3 || (dy > 12 && v > 0.45)) {
      open = false;
      onclose?.();
      hide(Math.max(0, dy));
    } else {
      const from = panel.style.transform || 'translateY(0)';
      panel.style.transform = '';
      panel.animate([{ transform: from }, { transform: 'translateY(0)' }], { duration: ms('--dur-sheet'), easing: ease() });
    }
  }
</script>

<dialog class="sheet" bind:this={dialog} aria-label={label} {oncancel} {onclick}>
  <div class="panel" bind:this={panel}>
    <div class="grab" onpointerdown={down} onpointermove={moveDrag} onpointerup={up} onpointercancel={up} role="presentation">
      <span class="handle" aria-hidden="true"></span>
      <div class="bar">
        {#if title}<h2 class="title">{label}</h2>{:else}<span></span>{/if}
        <IconButton icon="x" label="Close" tip="none" onclick={close} />
      </div>
    </div>
    <div class="body">{@render children()}</div>
    {#if footer}<div class="foot">{@render footer()}</div>{/if}
  </div>
</dialog>

<style>
  :global(html:has(dialog.sheet[open])) { overflow: hidden; }
  .sheet { position: fixed; inset: auto 0 0; margin: 0 auto; padding: 0; border: 0; width: 100%; max-width: 640px;
    max-height: 88dvh; background: transparent; color: var(--text); overflow: visible; }
  .sheet::backdrop { background: var(--scrim); }
  .panel { display: flex; flex-direction: column; max-height: 88dvh; border: 1px solid var(--border); border-bottom: 0;
    border-radius: var(--radius-card) var(--radius-card) 0 0; background: var(--elevated); box-shadow: var(--shadow-pop);
    padding-bottom: env(safe-area-inset-bottom, 0px); }
  .grab { flex: none; padding: 8px max(var(--page-pad), env(safe-area-inset-left, 0px)) 0; touch-action: none; cursor: grab;
    -webkit-user-select: none; user-select: none; }
  .handle { display: block; width: 36px; height: 4px; margin: 0 auto 4px; border-radius: 2px; background: var(--border-strong); }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; }
  .title { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; }
  .body { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain;
    padding: 8px max(var(--page-pad), env(safe-area-inset-right, 0px)) 20px max(var(--page-pad), env(safe-area-inset-left, 0px)); }
  .foot { flex: none; display: flex; gap: 12px; padding: 12px var(--page-pad) 16px; border-top: 1px solid var(--border); }
  .foot :global(.btn) { flex: 1; }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .panel { border-color: var(--border-strong); }
  }
</style>
