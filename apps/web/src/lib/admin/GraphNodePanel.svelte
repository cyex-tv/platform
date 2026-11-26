<script lang="ts">
  // The inspector of the entitlement graph's picked node: what it is, its facts as label/value
  // rows (ids in mono, dates in cyex format), the grants that run through it (how, when, by whom,
  // why), and where to go from it (the item's holders, an account's own graph, the order).
  import IconButton from '$lib/components/IconButton.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { dateTime } from '$lib/format';
  import AccountLink from './AccountLink.svelte';
  import { KIND_LABEL, SOURCE_LABEL, type Grant } from './graph';
  import { NODE_LABEL, type GNode, type Graph } from './entitlement-graph';

  let { node, graph, grants, labelOf, onclose }: { node: GNode; graph: Graph; grants: Grant[]; labelOf: (n: GNode) => string; onclose: () => void } = $props();

  const isItem = $derived(node.kind === 'role' || node.kind === 'badge' || node.kind === 'paint');
  const mine = $derived(node.grants.map((i) => grants[i]).filter(Boolean));
  const holdersView = $derived(!graph.byId.get(graph.root)?.accountId);
  // Grants are shown in full on the item (account view) or the holder (holders view); a source
  // node shows what came from it as counts instead.
  const detailed = $derived(!node.root && ((isItem && !holdersView) || (node.kind === 'account' && holdersView)));
  const links = $derived(graph.edges.filter((e) => e.source === node.id || e.target === node.id).length);
  const out = $derived.by(() => {
    const c: Record<string, number> = {};
    for (const e of graph.edges) if (e.source === node.id) {
      const k = graph.byId.get(e.target)?.kind;
      if (k) c[k] = (c[k] ?? 0) + 1;
    }
    return Object.entries(c) as [GNode['kind'], number][];
  });
  const itemHref = (kind: string, id: string) => `/admin/entitlements?${kind}=${encodeURIComponent(id)}`;
  const adminItem = (n: GNode) => (n.kind === 'role' ? `/admin/roles?role=${encodeURIComponent(n.itemId!)}` : `/admin/cosmetics/${n.kind}s?${n.kind}=${encodeURIComponent(n.itemId!)}`);
  const given = (g: Grant) => (g.grantedAt ? dateTime(g.grantedAt) : g.source === 'DEFAULT' || g.source === 'OWNER' ? 'Always' : 'Before the log');
  const plural = (k: GNode['kind'], n: number) => `${NODE_LABEL[k].toLowerCase()}${n === 1 ? '' : k === 'code' ? 'es' : 's'}`;
</script>

<div class="adm-ip-head">
  <span class="adm-label">{NODE_LABEL[node.kind]}{node.root ? ' · root' : ''}</span>
  <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={onclose} />
</div>
<div class="adm-ip-body">
  <section class="adm-sec">
    <h2 class="adm-name"><bdi>{labelOf(node) || 'Unknown'}</bdi></h2>
    {#if !node.root}<p class="adm-flags"><span class:ok={node.active} class:warn={!node.active}>{node.active ? 'Held now' : 'Taken away'}</span></p>{/if}
  </section>
  <section class="adm-sec">
    <h3 class="adm-label">Facts</h3>
    <dl class="adm-kv">
      <dt>Kind</dt><dd class="txt">{NODE_LABEL[node.kind]}</dd>
      {#if node.accountId}<dt>Account</dt><dd class="txt"><AccountLink id={node.accountId} /></dd><dt>Account ID</dt><dd class="mono">{node.accountId}</dd>{/if}
      {#if node.itemId}<dt>Item ID</dt><dd class="mono">{node.itemId}</dd>{/if}
      {#if node.orderId}<dt>Order</dt><dd class="txt"><a href="/admin/shop/orders?order={encodeURIComponent(node.orderId)}">{node.name}</a></dd><dt>Order ID</dt><dd class="mono">{node.orderId}</dd>
        {#if mine[0]?.orderStatus}<dt>Status</dt><dd class="txt">{mine[0].orderStatus}</dd>{/if}{/if}
      {#if node.codeBatchId}<dt>Code batch</dt><dd class="txt"><a href="/admin/shop/codes?batch={encodeURIComponent(node.codeBatchId)}">{node.name}</a></dd>{/if}
      <dt>Links</dt><dd>{links}</dd>
      <dt>Grants</dt><dd>{mine.length}</dd>
      {#each out as [k, n] (k)}<dt>{NODE_LABEL[k]}{n === 1 ? '' : 's'}</dt><dd>{n}</dd>{/each}
    </dl>
  </section>
  {#if detailed}
    {#each mine as g, i (i)}
      <section class="adm-sec">
        <h3 class="adm-label">{mine.length > 1 ? `How (${i + 1} of ${mine.length})` : 'How'}</h3>
        <dl class="adm-kv">
          <dt>What</dt><dd class="txt">{KIND_LABEL[g.kind]}: <bdi>{g.itemName || 'Deleted'}</bdi></dd>
          <dt>Source</dt><dd class="txt">{SOURCE_LABEL[g.source]}</dd>
          {#if g.orderName}<dt>Order</dt><dd class="txt">{#if g.orderId}<a href="/admin/shop/orders?order={encodeURIComponent(g.orderId)}">{g.orderName}</a>{:else}{g.orderName}{/if}</dd>{/if}
          {#if g.codeBatchName}<dt>Code batch</dt><dd class="txt"><a href="/admin/shop/codes{g.codeBatchId ? `?batch=${encodeURIComponent(g.codeBatchId)}` : ''}">{g.codeBatchName}</a></dd>{/if}
          {#if holdersView}<dt>Holder</dt><dd class="txt"><AccountLink id={g.accountId} /></dd>{/if}
          <dt>Given</dt><dd>{given(g)}</dd>
          {#if g.grantedBy}<dt>Given by</dt><dd class="txt"><AccountLink id={g.grantedBy} /></dd>{/if}
          {#if g.revokedAt}<dt>Taken away</dt><dd>{dateTime(g.revokedAt)}</dd>{/if}
          {#if g.revokedBy}<dt>Taken by</dt><dd class="txt"><AccountLink id={g.revokedBy} /></dd>{/if}
        </dl>
        {#if g.reason}<p class="rs"><span class="adm-label">Reason</span> <bdi>{g.reason}</bdi></p>{/if}
        {#if g.revokeReason}<p class="rs"><span class="adm-label">Why taken away</span> <bdi>{g.revokeReason}</bdi></p>{/if}
      </section>
    {/each}
  {/if}
  <section class="adm-sec">
    <div class="adm-acts row">
      {#if isItem && !node.root}
        <a class="ui-btn secondary small" href={itemHref(node.kind, node.itemId!)}><Icon name="network" size={14} />Who holds it</a>
        <a class="ui-btn ghost small" href={adminItem(node)}>Open {NODE_LABEL[node.kind].toLowerCase()}</a>
      {:else if isItem}
        <a class="ui-btn ghost small" href={adminItem(node)}>Open {NODE_LABEL[node.kind].toLowerCase()}</a>
      {/if}
      {#if node.accountId}
        {#if !node.root}<a class="ui-btn secondary small" href="/admin/entitlements?account={encodeURIComponent(node.accountId)}"><Icon name="network" size={14} />Their graph</a>{/if}
        <a class="ui-btn ghost small" href="/admin/users/{encodeURIComponent(node.accountId)}">Account page</a>
      {/if}
      {#if node.orderId}<a class="ui-btn ghost small" href="/admin/shop/orders?order={encodeURIComponent(node.orderId)}">Open order</a>{/if}
      {#if node.codeBatchId}<a class="ui-btn ghost small" href="/admin/shop/codes?batch={encodeURIComponent(node.codeBatchId)}">Open code batch</a>{/if}
      {#if node.kind === 'product'}<a class="ui-btn ghost small" href="/admin/shop/items">Shop items</a>{/if}
    </div>
    {#if !node.root && (isItem || node.accountId)}<p class="hint">Double-click it in the graph to open its own graph.</p>{/if}
    {#if out.length}<p class="hint">{out.map(([k, n]) => `${n} ${plural(k, n)}`).join(', ')} came from it.</p>{/if}
  </section>
</div>

<style>
  .rs { margin: 8px 0 0; font-size: var(--t-small); overflow-wrap: anywhere; }
  .rs .adm-label { margin-right: 6px; }
  .hint { margin: 8px 0 0; font-size: var(--t-fine); color: var(--dim); }
</style>
