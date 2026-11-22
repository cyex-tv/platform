<script lang="ts">
  // Test harness: the browse grid with Select mode and the picks kept here, the picks written
  // out (in order) so a test can read them, and the page's Esc (clear the picks, then leave).
  import BrowseGrid from '$lib/components/browse/BrowseGrid.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import { CAP, toggle, type Selection, type Step } from '$lib/components/browse/selection';

  let { items, picked = [], start = null, on = true }: { items: CardEmote[]; picked?: string[]; start?: string | null; on?: boolean } = $props();
  // svelte-ignore state_referenced_locally
  let selecting = $state(on);
  // svelte-ignore state_referenced_locally
  let sel = $state.raw<Selection>({ ids: picked, anchor: start });
  let note = $state('');
  function step(s: Step) {
    sel = s.sel;
    note = s.refused ? `That’s ${CAP}, the most at once.` : '';
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !selecting) return;
    if (sel.ids.length) sel = { ids: [], anchor: null };
    else selecting = false;
  }
</script>

<svelte:window {onkeydown} />
<BrowseGrid
  {items}
  label="Emotes"
  {selecting}
  {sel}
  onstep={step}
  onstart={(id) => {
    selecting = true;
    if (id && !sel.ids.includes(id)) step(toggle(sel, id));
  }}
  onstop={() => {
    selecting = false;
    sel = { ids: [], anchor: null };
  }}
/>
<output data-testid="picks">{sel.ids.join(',')}</output>
<output data-testid="anchor">{sel.anchor ?? ''}</output>
<output data-testid="mode">{selecting ? 'on' : 'off'}</output>
<output data-testid="note">{note}</output>
