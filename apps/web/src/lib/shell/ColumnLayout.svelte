<script lang="ts">
  // Page layout 2 of 2: one column in the full-width wrapper. `width` narrows the content inside the
  // wrapper (never a second wrapper): "page" (the full width), "measure" (820, prose) or "form"
  // (640, single tasks); the narrow ones are centred in the window. `top` adds the rail layout's 20px under the header, for dense pages.
  import type { Snippet } from 'svelte';

  type Props = { width?: 'page' | 'measure' | 'form'; top?: boolean; children: Snippet };
  let { width = 'page', top = false, children }: Props = $props();
</script>

<div class="wrap column-layout {width}" class:top>
  <div class="inner">{@render children()}</div>
</div>

<style>
  .top { padding-top: var(--rail-gap); }
  .measure .inner { max-width: var(--measure); margin-inline: auto; }
  .form .inner { max-width: var(--measure-form); margin-inline: auto; }
</style>
