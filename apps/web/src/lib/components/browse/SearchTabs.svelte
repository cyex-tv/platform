<script lang="ts">
  // Emotes | Users, the same switch on both tabs of /search. The name typed carries over; the
  // rest of the view starts fresh. A pick is a navigation, so Back returns to the other tab.
  import { goto } from '$app/navigation';
  import Segmented from '$lib/components/Segmented.svelte';
  import { change, toSearch, type Browse, type Tab } from './params';

  let { view }: { view: Browse } = $props();
  const tabs: { value: Tab; label: string }[] = [
    { value: 'emotes', label: 'Emotes' },
    { value: 'users', label: 'Users' },
  ];
</script>

<Segmented
  label="Search in"
  size="s"
  options={tabs}
  value={view.tab}
  onchange={(tab) => goto(`/search${toSearch(change(view, { tab }))}`, { noScroll: true })}
/>
