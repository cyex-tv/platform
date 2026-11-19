<script lang="ts">
  // Your subscription. There are no subscriptions on cyex.tv yet: the header shows this page only
  // to accounts with an active one (hasSubscription in lib/shell/nav.ts, false until they exist),
  // so for now it is a placeholder in the one empty-state pattern.
  import State from '$lib/components/State.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
</script>

<svelte:head><title>Subscription - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Subscription" />
  <State title="No subscriptions yet">
    Subscriptions aren’t available on cyex.tv yet. Once they are, your plan, renewals and receipts show up here.
    {#snippet actions()}<a class="ui-btn secondary" href="/shop">Go to the store</a>{/snippet}
  </State>
</div>
