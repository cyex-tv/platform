<script lang="ts">
  // Your subscription. There are no subscriptions on cyex.tv yet: the header shows this page only
  // to accounts with an active one (hasSubscription in lib/shell/nav.ts, false until they exist),
  // so for now it is a placeholder in the one empty-state pattern, in your profile's frame.
  import Icon from '$lib/components/Icon.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import State from '$lib/components/State.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
</script>

<svelte:head><title>Subscription - cyex.tv</title></svelte:head>

{#snippet side()}
  <PanelBlock title="Related">
    <p class="pb-hint">What you bought in the shop, with receipts, is under Billing in Settings.</p>
    <div class="pb-actions">
      <a class="ui-btn secondary" href="/me/settings/billing"><Icon name="shopping-bag" size={15} />Billing</a>
      <a class="ui-btn secondary" href="/shop"><Icon name="tag" size={15} />Store</a>
    </div>
  </PanelBlock>
{/snippet}

<MeFrame title="Subscription" label="About subscriptions" panel={side}>
  <div class="col">
    <PageHead title="Subscription" />
    <State title="No subscriptions yet">
      Subscriptions aren’t available on cyex.tv yet. Once they are, your plan, renewals and receipts show up here.
      {#snippet actions()}<a class="ui-btn secondary" href="/shop">Go to the store</a>{/snippet}
    </State>
  </div>
</MeFrame>

<style>
  .col { max-width: 760px; }
</style>
