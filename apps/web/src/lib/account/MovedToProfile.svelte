<script lang="ts">
  // An old Settings address (Account, Editors) whose page moved to your profile's owner-only tabs:
  // it opens the tab, the #hash kept (the (tool) layout signs you in first, so `session.me` is set).
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { movedToProfile } from './settings';

  onMount(() => {
    const to = session.me ? movedToProfile(page.url.pathname, session.me.id, page.url.hash) : null;
    if (to) void goto(to, { replaceState: true });
  });
</script>

<ColumnLayout top><State kind="loading" title="Opening your profile…" /></ColumnLayout>
