<script lang="ts">
  // /me/settings opens on Account.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import State from '$lib/components/State.svelte';

  onMount(() => goto('/me/settings/account', { replaceState: true }));
</script>

<State kind="loading" title="Opening your settings…" />
