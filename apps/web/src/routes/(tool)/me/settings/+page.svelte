<script lang="ts">
  // /me/settings opens on Appearance (Account moved to your profile).
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import State from '$lib/components/State.svelte';
  import { SETTINGS_HOME } from '$lib/account/settings';

  onMount(() => goto(SETTINGS_HOME, { replaceState: true }));
</script>

<State kind="loading" title="Opening your settings…" />
