<script lang="ts">
  // The old account page lives in Settings → Account now; this address still works and goes there.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import State from '$lib/components/State.svelte';

  onMount(() => goto(`/me/settings/account${page.url.hash}`, { replaceState: true }));
</script>

<State kind="loading" title="Opening your settings…" />
