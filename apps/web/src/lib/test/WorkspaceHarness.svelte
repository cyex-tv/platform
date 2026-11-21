<script lang="ts">
  // Test harness: the browse page's workspace without the route: the set rail, the grid (with
  // Select mode kept here) and the tray.
  import BrowseGrid from '$lib/components/browse/BrowseGrid.svelte';
  import SetRail from '$lib/components/browse/SetRail.svelte';
  import TargetTray from '$lib/components/browse/TargetTray.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import { EMPTY, type Selection } from '$lib/components/browse/selection';

  let { items }: { items: CardEmote[] } = $props();
  let selecting = $state(false);
  let sel = $state.raw<Selection>(EMPTY);
  let open = $state(true);
</script>

<SetRail />
<BrowseGrid
  {items}
  label="Emotes"
  {selecting}
  {sel}
  onstep={(s) => (sel = s.sel)}
  onstart={(id) => {
    selecting = true;
    if (id) sel = { ...sel, ids: [...sel.ids, id] };
  }}
/>
<TargetTray {open} ontoggle={() => (open = !open)} />
