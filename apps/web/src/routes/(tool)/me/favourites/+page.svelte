<script lang="ts">
  // Your favourites live on your profile (a private tab only you see): this address goes there.
  import { goto } from '$app/navigation';
  import State from '$lib/components/State.svelte';
  import { sectionHref } from '$lib/profile/rows';
  import { session } from '$lib/tool/session.svelte';

  $effect(() => {
    const id = session.me?.id;
    if (id) goto(sectionHref(id, 'favourites'), { replaceState: true });
  });
</script>

<State kind="loading" title="Opening your favourites…" />
