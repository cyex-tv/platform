<script lang="ts">
  // A name with its badge and paint on a chat ground, in a theme of its own (data-theme scopes
  // the tokens, so the dark and the light ground sit side by side whichever theme the page is in).
  // Flat (no box of its own: the caller's frame has the hairlines); `tag` names the ground.
  // `line` shows it as a chat line ("name: hi chat"); otherwise the name alone, large.
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import type { Badge, Paint } from '$lib/cosmetics/store.svelte';

  type Props = {
    name: string;
    paint?: Paint | null;
    badge?: Badge | null;
    theme?: 'dark' | 'light';
    line?: boolean;
    /** A small uppercase word in the corner (Dark, Light), in the sample's own theme. */
    tag?: string;
    /** What the screen reader hears instead of the decorative sample. */
    label: string;
  };
  let { name, paint = null, badge = null, theme, line = false, tag, label }: Props = $props();
</script>

<div class="sample" class:line data-theme={theme} role="img" aria-label={label}>
  {#if tag}<span class="tag" aria-hidden="true">{tag}</span>{/if}
  {#if line}
    <span class="chatline" aria-hidden="true">
      <PaintedName {name} {paint} {badge} /><span class="msg">: hi chat</span>
    </span>
  {:else}
    <span class="big" aria-hidden="true"><PaintedName {name} {paint} {badge} badgeSize={20} /></span>
  {/if}
</div>

<style>
  /* Flat: the frame around it draws the hairlines. */
  .sample { position: relative; display: flex; align-items: center; min-width: 0; min-height: 76px; padding: 14px 12px 0; background: var(--bg); color: var(--text); }
  .line { min-height: 44px; padding-top: 0; }
  .tag { position: absolute; top: 7px; left: 10px; font-size: 10px; font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .big { display: flex; min-width: 0; font-size: 18px; font-weight: 700; letter-spacing: -.01em; }
  .chatline { display: flex; align-items: center; min-width: 0; font-size: 13px; font-weight: 700; }
  .chatline :global(.pn) { flex: 0 1 auto; }
  .msg { flex: none; font-weight: 400; color: var(--muted); white-space: nowrap; }
</style>
