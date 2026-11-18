<script lang="ts">
  // Emotes | Users, the same switch on both tabs of /search. The name typed carries over; the
  // rest of the view starts fresh. A pick is a navigation, so Back returns to the other tab.
  import { goto } from '$app/navigation';
  import type { IconName } from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { change, toSearch, type Browse, type Tab } from './params';

  let { view }: { view: Browse } = $props();
  const tabs: { value: Tab; label: string; icon: IconName }[] = [
    { value: 'emotes', label: 'Emotes', icon: 'smile' },
    { value: 'users', label: 'Users', icon: 'users' },
  ];
</script>

<Segmented
  label="Search in"
  size="s"
  options={tabs}
  value={view.tab}
  onchange={(tab) => goto(`/search${toSearch(change(view, { tab }))}`, { noScroll: true })}
/>
