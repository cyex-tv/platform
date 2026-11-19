<script lang="ts">
  // Settings → Billing: your orders from the shop (receipts, gifts, codes).
  import OrdersPanel from '$lib/shop/OrdersPanel.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
</script>

<svelte:head><title>Billing - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Billing" line="What you bought in the shop, with receipts." />
  <OrdersPanel />
</div>
