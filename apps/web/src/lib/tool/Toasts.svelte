<script lang="ts">
  import { dismiss, toasts } from './notices.svelte';

  const notices = $derived(toasts.filter((t) => !t.error));
  const errors = $derived(toasts.filter((t) => t.error));
</script>

{#snippet list(items: typeof toasts)}
  {#each items as t (t.id)}
    <div class="toast" class:error={t.error} role={t.error ? 'alert' : 'status'}>
      <p>{t.text}{#if t.count > 1}<span class="count ui-mono">×{t.count}</span>{/if}</p>
      <button class="close" type="button" aria-label="Dismiss" onclick={() => dismiss(t.id)}>
        <svg viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18" /></svg>
      </button>
    </div>
  {/each}
{/snippet}

<div class="stack notices" aria-live="polite">{@render list(notices)}</div>
<div class="stack errors" aria-live="assertive">{@render list(errors)}</div>

<style>
  .stack { position: fixed; bottom: 18px; z-index: var(--z-toast); display: grid; gap: 8px; width: min(380px, calc(100vw - 36px)); }
  .notices { right: 18px; }
  .errors { left: 18px; }
  .toast { position: relative; padding: 12px 34px 13px 14px; background: var(--elevated); border: 1px solid var(--border-strong);
    border-left: 3px solid var(--border-strong); box-shadow: var(--shadow-pop); font-size: .85rem;
    animation: in var(--base) var(--ease); }
  .toast.error { border-left-color: var(--red); }
  .notices .toast { --from: 16px; }
  .errors .toast { --from: -16px; }
  @keyframes in { from { opacity: 0; translate: var(--from) 0; scale: .98; } }
  .count { margin-left: 6px; color: var(--dim); }
  .close { position: absolute; top: 8px; right: 8px; width: 18px; height: 18px; display: grid; place-items: center; padding: 0;
    border: 0; border-radius: 50%; background: var(--border-strong); color: var(--text); cursor: pointer; }
  .close:active { opacity: .6; }
  .close svg { fill: none; stroke: currentColor; stroke-width: 2.2; stroke-linecap: round; }
  @media (prefers-reduced-motion: reduce) { .toast { animation: none; } }
</style>
