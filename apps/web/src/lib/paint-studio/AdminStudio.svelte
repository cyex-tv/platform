<script lang="ts">
  // The paint studio inside the admin pages: needs cosmetics.manage, saves to the live API, and
  // after the first save of a new paint its address becomes that paint's own, so a reload opens it.
  import { replaceState } from '$app/navigation';
  import State from '$lib/components/State.svelte';
  import { forgetCatalog } from '$lib/cosmetics/store.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { apiBackend, type SavedPaint } from './backend';
  import PaintStudio from './PaintStudio.svelte';

  let { paintId = null }: { paintId?: string | null } = $props();
  const backend = apiBackend();
  let savedId = $state<string | null>(null);
  const editing = $derived(Boolean(savedId ?? paintId));

  function saved(s: SavedPaint) {
    forgetCatalog();
    notify(`${s.paint.name} saved.`);
    // the address follows the paint without reloading the studio (shallow routing)
    const was = savedId ?? paintId;
    savedId = s.id;
    if (s.id !== was) replaceState(`/admin/cosmetics/paints/${encodeURIComponent(s.id)}/studio`, {});
  }
</script>

<svelte:head><title>{editing ? 'Edit paint' : 'New paint'} - cyex.tv</title></svelte:head>

{#if !can('cosmetics.manage')}
  <State level={1} role="status" title="No access here.">
    Making and changing paints needs cosmetics.manage.
    {#snippet actions()}<a class="ui-btn secondary" href="/admin/cosmetics/paints">Back to paints</a>{/snippet}
  </State>
{:else}
  <PageHead
    title={editing ? 'Edit paint' : 'New paint'}
    line="Gradients and animated pictures, checked against what cyex.tv accepts as you go."
    back={{ href: '/admin/cosmetics/paints', label: 'Paints' }}
  />
  <!-- read once on mount: after the first save the address changes, the open paint stays -->
  <PaintStudio {backend} {paintId} onsaved={saved} />
{/if}
