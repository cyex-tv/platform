<script lang="ts">
  // Your sets now live on your profile (your profile is your dashboard): this address still works and goes there.
  // `?owner=<id>` opens the account you edit instead of yours.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { sectionHref } from '$lib/profile/rows';
  import { session } from '$lib/tool/session.svelte';

  $effect(() => {
    const id = page.url.searchParams.get('owner') || session.me?.id;
    if (id) goto(sectionHref(id, 'sets'), { replaceState: true });
  });
</script>

<ColumnLayout top><State kind="loading" title="Opening your profile…" /></ColumnLayout>
