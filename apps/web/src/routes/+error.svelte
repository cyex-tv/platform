<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Problem from '$lib/components/Problem.svelte';
  import { causeOfStatus, problemTitle } from '$lib/components/problem';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  const cause = $derived(causeOfStatus(page.status));
</script>

<svelte:head>
  <title>{problemTitle(cause)}</title>
</svelte:head>

<ColumnLayout top>
  <Problem {cause} level={1} onretry={() => invalidateAll()} />
</ColumnLayout>
