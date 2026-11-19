<script lang="ts">
  // A name with its badge and paint on a chat ground, in a theme of its own (data-theme scopes
  // the tokens, so the dark and the light ground sit side by side whichever theme the page is in).
  // `line` shows it as a chat line ("name: hi chat"); otherwise the name alone, large.
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import type { Badge, Paint } from '$lib/cosmetics/store.svelte';

  type Props = {
    name: string;
    paint?: Paint | null;
    badge?: Badge | null;
    theme?: 'dark' | 'light';
    line?: boolean;
    /** What the screen reader hears instead of the decorative sample. */
    label: string;
  };
  let { name, paint = null, badge = null, theme, line = false, label }: Props = $props();
</script>

<div class="sample" class:line data-theme={theme} role="img" aria-label={label}>
  {#if line}
    <span class="chatline" aria-hidden="true">
      <PaintedName {name} {paint} {badge} /><span class="msg">: hi chat</span>
    </span>
  {:else}
    <span class="big" aria-hidden="true"><PaintedName {name} {paint} {badge} badgeSize={20} /></span>
  {/if}
</div>

<style>
  .sample { display: flex; align-items: center; min-width: 0; min-height: 52px; padding: 0 12px; border-radius: var(--radius); background: var(--bg); color: var(--text);
    border: 1px solid var(--border); }
  .line { min-height: 40px; }
  .big { display: flex; min-width: 0; font-size: 18px; font-weight: 700; letter-spacing: -.01em; }
  .chatline { display: flex; align-items: center; min-width: 0; font-size: 13px; font-weight: 700; }
  .chatline :global(.pn) { flex: 0 1 auto; }
  .msg { flex: none; font-weight: 400; color: var(--muted); white-space: nowrap; }
</style>
