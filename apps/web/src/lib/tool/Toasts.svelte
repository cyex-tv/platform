<script lang="ts">
  // The toast stacks (cyex-app web-tool toasts): notices bottom right, errors bottom left. Mounted
  // by the tool layout and by the root layout's public branch, so it reads the tool tokens with
  // the web tokens as fallbacks. A toast slides in from its side, fades out, and the rest of the
  // stack glides into place; none of it moves under reduced motion.
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import { dur } from '$lib/motion';
  import { barNodes, barsVersion, lift } from './bottom-bars.svelte';
  import { dismiss, pauseToast, resumeToast, toasts } from './notices.svelte';

  const notices = $derived(toasts.filter((t) => !t.error));
  const errors = $derived(toasts.filter((t) => t.error));

  // Each stack is a manual popover, so it lives in the top layer and paints above an open modal
  // <dialog> (a z-index never can). It shows while it has toasts; every new toast or repeat shows
  // it again, which moves it to the end of the top layer, above a dialog opened after it.
  function layer(node: HTMLElement, version: number) {
    let hide = 0;
    const sync = (v: number) => {
      clearTimeout(hide);
      if (v > 0) {
        if (node.matches(':popover-open')) node.hidePopover();
        node.showPopover();
      } else hide = window.setTimeout(() => node.matches(':popover-open') && node.hidePopover(), 260); // after the fade-out
    };
    sync(version);
    return {
      update: sync,
      destroy() {
        clearTimeout(hide);
        if (node.matches(':popover-open')) node.hidePopover();
      },
    };
  }
  const stamp = (items: typeof toasts) => items.reduce((n, t) => n + t.id * 7 + t.count, 0) * (items.length ? 1 : 0);

  // Bottom bars (the select bar, the emote page's phone bar) push the stacks up so a toast never
  // covers their buttons. Measured only while a toast shows, on scroll, resize and bar changes.
  let raise = $state(0);
  $effect(() => {
    barsVersion();
    if (!toasts.length) return;
    let frame = 0;
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => (raise = lift(barNodes().map((n) => n.getBoundingClientRect()), innerHeight)));
    };
    measure();
    const ro = new ResizeObserver(measure);
    for (const n of barNodes()) ro.observe(n);
    addEventListener('scroll', measure, { passive: true, capture: true });
    addEventListener('resize', measure, { passive: true });
    return () => {
      cancelAnimationFrame(frame);
      ro.disconnect();
      removeEventListener('scroll', measure, { capture: true });
      removeEventListener('resize', measure);
    };
  });
</script>

{#snippet list(items: typeof toasts, from: number)}
  {#each items as t (t.id)}
    <div
      class="toast"
      class:error={t.error}
      role={t.error ? 'alert' : 'status'}
      in:fly={{ x: from, duration: dur(200), opacity: 0 }}
      out:fade={{ duration: dur(120) }}
      animate:flip={{ duration: dur(200) }}
      onpointerenter={() => pauseToast(t.id)}
      onpointerleave={() => resumeToast(t.id)}
      onfocusin={() => pauseToast(t.id, 'focus')}
      onfocusout={() => resumeToast(t.id, 'focus')}
    >
      <p>{t.text}{#if t.count > 1}<span class="count">×{t.count}</span>{/if}{#if t.action}{@const act = t.action}{#if act.href}<a class="act" href={typeof act.href === 'function' ? act.href() : act.href} onclick={(e) => { if (typeof act.href === 'function') e.currentTarget.href = act.href(); dismiss(t.id); act.run?.(); }}>{act.label}</a>{:else}<button class="act" type="button" onclick={() => { dismiss(t.id); act.run?.(); }}>{act.label}</button>{/if}{/if}</p>
      {#if t.ms}<span class="line" style:animation-duration="{t.ms}ms" aria-hidden="true"></span>{/if}
      <button class="close" type="button" aria-label="Dismiss" onclick={() => dismiss(t.id)}>
        <svg viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18" /></svg>
      </button>
    </div>
  {/each}
{/snippet}

<div class="stack notices" popover="manual" use:layer={stamp(notices)} style:--raise="{raise}px" aria-live="polite">{@render list(notices, 16)}</div>
<div class="stack errors" popover="manual" use:layer={stamp(errors)} style:--raise="{raise}px" aria-live="assertive">{@render list(errors, -16)}</div>

<style>
  .stack { position: fixed; z-index: var(--z-toast); gap: 8px; width: min(380px, calc(100vw - 36px));
    inset: auto; margin: 0; padding: 0; border: 0; background: transparent; color: inherit; overflow: visible;
    bottom: max(calc(18px + env(safe-area-inset-bottom, 0px)), calc(var(--raise, 0px) + 10px)); }
  .stack:popover-open { display: grid; }
  .notices { right: 18px; left: auto; }
  .errors { left: 18px; right: auto; }
  .toast { position: relative; padding: 12px 34px 13px 14px; background: var(--elevated, var(--surface-2)); color: var(--text);
    border: 1px solid var(--border-strong); border-left: 3px solid var(--border-strong); box-shadow: var(--shadow-pop);
    font-size: .85rem; line-height: 1.45; }
  /* The Undo countdown: a hairline along the bottom that runs down over the wait. It follows the
     clock, so it is not held by the pointer. Under reduced motion it ends at once and is gone. */
  .line { position: absolute; left: 0; right: 0; bottom: 0; height: 2px; background: var(--accent); transform-origin: 0 50%;
    animation: undo-line linear forwards; }
  @keyframes undo-line { to { transform: scaleX(0); } }
  .toast.error { border-left-color: var(--red, var(--text)); }
  .toast p { margin: 0; overflow-wrap: anywhere; }
  .act { margin-left: 10px; padding: 0 2px; min-height: 24px; border: 0; background: none; color: var(--accent-text, var(--accent)); font: inherit; font-weight: 600; cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
  .act:focus-visible { outline: 2px solid var(--focus-ring, var(--accent)); outline-offset: 2px; border-radius: var(--radius); }
  .count { margin-left: 6px; color: var(--dim, var(--muted)); font-family: var(--font-mono, inherit); font-variant-numeric: tabular-nums; }
  .close { position: absolute; top: 8px; right: 8px; width: 18px; height: 18px; display: grid; place-items: center; padding: 0;
    border: 0; border-radius: 50%; background: var(--border-strong); color: var(--text); cursor: pointer; }
  .close::after { content: ''; position: absolute; inset: -11px; } /* a 40px hit area around the small knob */
  .close:active { opacity: .6; }
  .close svg { fill: none; stroke: currentColor; stroke-width: 2.2; stroke-linecap: round; }
</style>
