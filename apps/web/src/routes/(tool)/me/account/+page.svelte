<script lang="ts">
  // The old account page lives in Settings → Account now; this address still works and goes there.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  onMount(() => goto(`/me/settings/account${page.url.hash}`, { replaceState: true }));
</script>

<ColumnLayout top><State kind="loading" title="Opening your settings…" /></ColumnLayout>
