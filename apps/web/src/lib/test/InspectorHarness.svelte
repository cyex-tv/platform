<script lang="ts">
  // InspectorLayout with a head, link tabs, content and a panel of PanelBlocks (one folded), for the shape tests.
  import LinkTabs from '$lib/components/LinkTabs.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';

  let { fold = false }: { fold?: boolean } = $props();
</script>

<InspectorLayout title="Set" label="About Main" {fold} testid="panel">
  {#snippet head()}<h1>Main</h1>{/snippet}
  {#snippet tabs()}
    <LinkTabs label="Sections" items={[{ href: '/a', label: 'Emotes', count: 24, current: true }, { href: '/b', label: 'Activity' }]} />
  {/snippet}
  <p>The grid</p>
  {#snippet panel()}
    <PanelBlock title="Facts"><dl class="kv"><dt>Emotes</dt><dd>24</dd></dl></PanelBlock>
    <PanelBlock title="Editors" count={3} fold more={{ href: '/me/editors', label: 'Manage' }}><p>nora_byte</p></PanelBlock>
  {/snippet}
</InspectorLayout>
