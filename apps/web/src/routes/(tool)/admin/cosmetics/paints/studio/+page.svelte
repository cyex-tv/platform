<script lang="ts">
  // The paint studio (making and editing paints) comes in its own change; this keeps its address.
  import { page } from '$app/state';
  import State from '$lib/components/State.svelte';
  import { can } from '$lib/tool/session.svelte';

  const editing = $derived(page.url.searchParams.get('id'));
</script>

<svelte:head><title>Paint studio - cyex.tv</title></svelte:head>

{#if !can('cosmetics.manage')}
  <State level={1} role="status" title="No access here.">Making paints needs cosmetics.manage.</State>
{:else}
  <State level={1} role="status" title="The paint studio is coming.">
    {editing ? 'Editing this paint' : 'Making new paints'} happens here soon. The paints that exist can be given and taken away from the list.
    {#snippet actions()}<a class="ui-btn secondary" href="/admin/cosmetics/paints">Back to paints</a>{/snippet}
  </State>
{/if}
