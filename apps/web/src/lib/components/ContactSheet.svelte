<script lang="ts">
  // A contact sheet (redesign round 2): cells that share hairlines in one sheet, no gaps and no
  // cards. Every direct child is a cell (`<li>` with the default `tag="ul"`). `min` is the
  // smallest cell width; cells stretch to fill the row. Emote grids use EmoteGrid / BrowseGrid
  // `sheet` (the same `.contact-sheet` rules from app.css); use this for sets, channels, files.
  //   <ContactSheet label="Your sets" min={220}>{#each sets as s}<li>…</li>{/each}</ContactSheet>
  import type { Snippet } from 'svelte';

  type Props = { label?: string; min?: number; tag?: 'ul' | 'div'; busy?: boolean; children: Snippet };
  let { label, min = 180, tag = 'ul', busy, children }: Props = $props();
</script>

<svelte:element this={tag} class="contact-sheet cs" style:--cs-min="{min}px" aria-label={label} aria-busy={busy}>{@render children()}</svelte:element>

<style>
  .cs { list-style: none; margin: 0; padding: 0; grid-template-columns: repeat(auto-fill, minmax(min(var(--cs-min), 100%), 1fr)); }
</style>
