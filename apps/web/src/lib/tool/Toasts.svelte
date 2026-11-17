<script lang="ts">
  // The toast stacks (cyex-app web-tool toasts): notices bottom right, errors bottom left. Mounted
  // by the tool layout and by the root layout's public branch, so it reads the tool tokens with
  // the web tokens as fallbacks. A toast slides in from its side, fades out, and the rest of the
  // stack glides into place; none of it moves under reduced motion.
  import { flip } from 'svelte/animate';
  import { fade, fly } from 'svelte/transition';
  import { dur } from '$lib/motion';
  import { dismiss, toasts } from './notices.svelte';

  const notices = $derived(toasts.filter((t) => !t.error));
  const errors = $derived(toasts.filter((t) => t.error));
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
    >
      <p>{t.text}{#if t.count > 1}<span class="count">×{t.count}</span>{/if}</p>
      <button class="close" type="button" aria-label="Dismiss" onclick={() => dismiss(t.id)}>
        <svg viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18" /></svg>
      </button>
    </div>
  {/each}
{/snippet}

<div class="stack notices" aria-live="polite">{@render list(notices, 16)}</div>
<div class="stack errors" aria-live="assertive">{@render list(errors, -16)}</div>

<style>
  .stack { position: fixed; bottom: calc(18px + env(safe-area-inset-bottom, 0px)); z-index: var(--z-toast); display: grid; gap: 8px;
    width: min(380px, calc(100vw - 36px)); }
  .notices { right: 18px; }
  .errors { left: 18px; }
  .toast { position: relative; padding: 12px 34px 13px 14px; background: var(--elevated, var(--surface-2)); color: var(--text);
    border: 1px solid var(--border-strong); border-left: 3px solid var(--border-strong); box-shadow: var(--shadow-pop);
    font-size: .85rem; line-height: 1.45; }
  .toast.error { border-left-color: var(--red, var(--text)); }
  .toast p { margin: 0; overflow-wrap: anywhere; }
  .count { margin-left: 6px; color: var(--dim, var(--muted)); font-family: var(--font-mono, inherit); font-variant-numeric: tabular-nums; }
  .close { position: absolute; top: 8px; right: 8px; width: 18px; height: 18px; display: grid; place-items: center; padding: 0;
    border: 0; border-radius: 50%; background: var(--border-strong); color: var(--text); cursor: pointer; }
  .close::after { content: ''; position: absolute; inset: -8px; } /* a 34px hit area around the small knob */
  .close:active { opacity: .6; }
  .close svg { fill: none; stroke: currentColor; stroke-width: 2.2; stroke-linecap: round; }
</style>
