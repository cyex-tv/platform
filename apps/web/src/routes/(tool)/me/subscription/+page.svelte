<script lang="ts">
  // Your subscription. There are no subscriptions on cyex.tv yet: the header shows this page only
  // to accounts with an active one (hasSubscription in lib/shell/nav.ts, false until they exist),
  // so for now it is a placeholder in the one empty-state pattern, in your profile's frame: the
  // panel holds the plan's facts (none yet) and links to Billing and the Shop.
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import State from '$lib/components/State.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
</script>

<svelte:head><title>Subscription - cyex.tv</title></svelte:head>

{#snippet side()}
  <PanelBlock title="Plan">
    <FactRows items={[
      { label: 'Plan', value: 'None' },
      { label: 'Renews', value: '—', mono: true },
    ]} />
  </PanelBlock>
  <PanelBlock title="Related">
    <LinkRows items={[
      { href: '/me/settings/billing', label: 'Billing', hint: 'What you bought, with receipts', icon: 'shopping-bag' },
      { href: '/shop', label: 'Shop', icon: 'tag' },
    ]} />
  </PanelBlock>
{/snippet}

{#snippet head()}<PageHead title="Subscription" line="Your plan and its renewals." />{/snippet}

<MeFrame title="Subscription" label="About subscriptions" panel={side} {head}>
  <State bare title="No subscriptions yet">
    Subscriptions aren’t available on cyex.tv yet. Once they are, your plan, renewals and receipts show up here.
    {#snippet actions()}<a class="ui-btn secondary" href="/shop">Go to the shop</a>{/snippet}
  </State>
</MeFrame>
