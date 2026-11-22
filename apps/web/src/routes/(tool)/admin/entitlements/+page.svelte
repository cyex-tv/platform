<script lang="ts">
  // The entitlement graph (7TV's, in cyex design): for an account, how each badge, paint and
  // role it holds or held was given, by whom and when; for a badge, paint or role, who holds it
  // and how. Open it from an account page, or pick an item here.
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import type { GrantKind } from '$lib/gql/graphql';
  import { ACCOUNT, ALL_ROLES, COSMETICS, ENTITLEMENT_GRAPH, ENTITLEMENT_HOLDERS } from '$lib/queries';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { looksLikeId } from '$lib/admin/audit';
  import EntitlementTree from '$lib/admin/EntitlementTree.svelte';
  import type { Grant } from '$lib/admin/graph';
  import { clean } from '$lib/format';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import { pickInUrl } from '$lib/admin/ui/pick';
  import { KIND_LABEL, SOURCE_LABEL, tally } from '$lib/admin/graph';
  import { dateTime } from '$lib/format';
  import { loader } from '$lib/tool/loader.svelte';
  import { can } from '$lib/tool/session.svelte';

  const allowed = can('entitlements.view') || can('cosmetics.grant');
  type Pick = 'BADGE' | 'PAINT' | 'ROLE';
  const params = $derived(page.url.searchParams);
  const target = $derived.by(() => {
    const account = params.get('account');
    if (account) return { kind: 'ACCOUNT' as const, id: account };
    for (const k of ['badge', 'paint', 'role'] as const) {
      const id = params.get(k);
      if (id) return { kind: k.toUpperCase() as Pick, id };
    }
    return null;
  });

  // The pickers: an account by ID, or an item from the catalog.
  let accountInput = $state('');
  let kind = $state<Pick>('BADGE');
  let item = $state('');
  let options = $state<{ BADGE: { id: string; name: string }[]; PAINT: { id: string; name: string }[]; ROLE: { id: string; name: string }[] }>({ BADGE: [], PAINT: [], ROLE: [] });
  $effect(() => {
    if (!allowed) return;
    gql(COSMETICS, {})
      .then((r) => (options = { ...options, BADGE: r.cosmetics.badges.map((b) => ({ id: b.id, name: b.name })), PAINT: r.cosmetics.paints.map((p) => ({ id: p.id, name: p.name })) }))
      .catch(() => {});
    if (can('roles.manage') || can('roles.assign'))
      gql(ALL_ROLES, {})
        .then((r) => (options = { ...options, ROLE: r.roles.filter((x) => !x.builtIn || x.builtIn === 'ADMIN').map((x) => ({ id: x.id, name: x.name })) }))
        .catch(() => {});
  });
  const list = $derived(options[kind]);
  $effect(() => {
    if (!list.some((o) => o.id === item)) item = list[0]?.id ?? '';
  });

  function openAccount(e: SubmitEvent) {
    e.preventDefault();
    const id = accountInput.trim().split('/').filter(Boolean).pop() ?? '';
    if (looksLikeId(id)) goto(`?account=${id.toUpperCase()}`);
  }
  function openItem(e: SubmitEvent) {
    e.preventDefault();
    if (item) goto(`?${kind.toLowerCase()}=${encodeURIComponent(item)}`);
  }

  const L = loader(async ({ fetch }) => {
    const t = target;
    if (!t || !allowed) return null;
    if (t.kind === 'ACCOUNT') {
      const [g, a] = await Promise.all([gql(ENTITLEMENT_GRAPH, { accountId: t.id }, fetch), gql(ACCOUNT, { id: t.id }, fetch)]);
      return { by: 'source' as const, root: clean(a.account?.displayName) || t.id, grants: g.entitlementGraph as Grant[], accountId: t.id };
    }
    const g = await gql(ENTITLEMENT_HOLDERS, { kind: t.kind as GrantKind, id: t.id }, fetch);
    const grants = g.entitlementHolders as Grant[];
    const name = grants.find((x) => x.itemName)?.itemName || options[t.kind].find((o) => o.id === t.id)?.name || t.id;
    return { by: 'account' as const, root: `${t.kind === 'ROLE' ? 'Role' : t.kind === 'BADGE' ? 'Badge' : 'Paint'}: ${name}`, grants, accountId: null };
  });
  // Show everything or what is held now, the graph or a list; the picked grant (?node=<n>) is in the address.
  let show = $state<'all' | 'held'>('all');
  let view = $state<'tree' | 'list'>('tree');
  const nodeParam = $derived(params.get('node'));
  const nodeNo = $derived(nodeParam !== null && /^\d{1,6}$/.test(nodeParam) ? Number(nodeParam) : null);
  const pickedGrant = $derived(nodeNo !== null ? L.data?.grants[nodeNo] ?? null : null);
  const sum = $derived(L.data ? tally(L.data.grants) : null);
  const pickNode = (n: number | null) => void pickInUrl('node', n === null ? null : String(n));
  const itemHref = (g: Grant) => `/admin/entitlements?${g.kind.toLowerCase()}=${encodeURIComponent(g.itemId)}`;

</script>

<svelte:head><title>Entitlement graph - Admin - cyex.tv</title></svelte:head>

{#if !allowed}
  <Bar title="Entitlement graph" />
  <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
{:else}
<Split label="Grant" open={!!pickedGrant} onclose={() => pickNode(null)}>
  <Bar title="Entitlement graph" count={L.data ? `${L.data.grants.length} ${L.data.grants.length === 1 ? 'grant' : 'grants'}` : null}>
    {#snippet tools()}
      <form class="grp" role="search" onsubmit={openAccount}>
        <label class="adm-find acc">
          <Icon name="user" size={15} />
          <span class="sr-only">An account, by ID or its admin page link</span>
          <input class="ui-input ui-mono" bind:value={accountInput} placeholder="Account ID or admin link" autocomplete="off" spellcheck="false" />
        </label>
        <button class="ui-btn secondary small" type="submit" disabled={!looksLikeId(accountInput.trim().split('/').filter(Boolean).pop() ?? '')}>Open</button>
        <a class="ui-btn ghost small" href="/admin/users">Find by name</a>
      </form>
      <form class="grp" onsubmit={openItem}>
        <Segmented label="Kind" size="s" compact options={[{ value: 'BADGE', label: 'Badge' }, { value: 'PAINT', label: 'Paint' }, ...(options.ROLE.length ? [{ value: 'ROLE' as const, label: 'Role' }] : [])]} value={kind} onchange={(v) => (kind = v)} />
        <label class="which">
          <span class="sr-only">Which {kind.toLowerCase()}</span>
          <select class="ui-input" bind:value={item} disabled={!list.length}>
            {#each list as o (o.id)}<option value={o.id}>{o.name}</option>{:else}<option value="">None yet</option>{/each}
          </select>
        </label>
        <button class="ui-btn secondary small" type="submit" disabled={!item}>Show holders</button>
      </form>
      {#if target}
        <div class="grp end">
          <Segmented label="Show" size="s" options={[{ value: 'all', label: 'Everything' }, { value: 'held', label: 'Held now' }]} value={show} onchange={(v) => (show = v)} />
          <Segmented label="View" size="s" options={[{ value: 'tree', label: 'Graph', icon: 'network' }, { value: 'list', label: 'List', icon: 'list' }]} value={view} onchange={(v) => (view = v)} />
        </div>
      {/if}
    {/snippet}
  </Bar>

  {#if target}
    <LoadGate source={L} label="Drawing the graph…">
      {#snippet children(d)}
        {#if d}
          <EntitlementTree grants={d.grants} root={d.root} by={d.by} {show} {view} picked={nodeNo} onpick={pickNode} />
        {/if}
      {/snippet}
    </LoadGate>
  {:else}
    <State bare title="Pick an account or an item.">Every account page links here too.</State>
  {/if}

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Graph</span></div>
    {#if L.data && sum}
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi>{L.data.root}</bdi></h2>
          {#if L.data.accountId}<a class="ui-btn ghost small" href="/admin/users/{encodeURIComponent(L.data.accountId)}">Account page</a>{/if}
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Held now</h3>
          <dl class="adm-kv">
            {#if L.data.by === 'source'}
              <dt>Roles</dt><dd>{sum.roles}</dd><dt>Badges</dt><dd>{sum.badges}</dd><dt>Paints</dt><dd>{sum.paints}</dd>
            {:else}
              <dt>Holders</dt><dd>{L.data.grants.length - sum.gone}</dd>
            {/if}
            <dt>Taken away</dt><dd>{sum.gone}</dd>
          </dl>
        </section>
      </div>
    {:else}
      <p class="none">Pick an account or an item, then a grant to see who gave it and when.</p>
    {/if}
  {/snippet}

  {#snippet panel()}
    {#if pickedGrant}
      {@const g = pickedGrant}
      <div class="adm-ip-head">
        <span class="adm-label">{KIND_LABEL[g.kind]}</span>
        <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={() => pickNode(null)} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi>{g.itemName || 'Deleted'}</bdi></h2>
          <p class="adm-flags"><span class:ok={g.active} class:warn={!g.active}>{g.active ? 'Held now' : 'Taken away'}</span></p>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">How</h3>
          <dl class="adm-kv">
            <dt>Source</dt><dd class="txt">{SOURCE_LABEL[g.source]}</dd>
            {#if g.orderName}<dt>Order</dt><dd class="txt">{#if g.orderId}<a href="/admin/shop/orders?order={encodeURIComponent(g.orderId)}">{g.orderName}</a>{:else}{g.orderName}{/if}</dd>{/if}
            {#if g.codeBatchName}<dt>Code batch</dt><dd class="txt"><a href="/admin/shop/codes{g.codeBatchId ? `?batch=${encodeURIComponent(g.codeBatchId)}` : ''}">{g.codeBatchName}</a></dd>{/if}
            <dt>Holder</dt><dd class="txt"><AccountLink id={g.accountId} /></dd>
            <dt>Given</dt><dd>{g.grantedAt ? dateTime(g.grantedAt) : g.source === 'DEFAULT' || g.source === 'OWNER' ? 'Always' : 'Before the log'}</dd>
            {#if g.grantedBy}<dt>Given by</dt><dd class="txt"><AccountLink id={g.grantedBy} /></dd>{/if}
            {#if g.revokedAt}<dt>Taken away</dt><dd>{dateTime(g.revokedAt)}</dd>{/if}
            {#if g.revokedBy}<dt>Taken by</dt><dd class="txt"><AccountLink id={g.revokedBy} /></dd>{/if}
            <dt>Item ID</dt><dd class="mono">{g.itemId}</dd>
          </dl>
        </section>
        {#if g.reason}
          <section class="adm-sec"><h3 class="adm-label">Reason</h3><p class="rs"><bdi>{g.reason}</bdi></p></section>
        {/if}
        {#if g.revokeReason}
          <section class="adm-sec"><h3 class="adm-label">Why it was taken away</h3><p class="rs"><bdi>{g.revokeReason}</bdi></p></section>
        {/if}
        <section class="adm-sec">
          <div class="adm-acts row">
            <a class="ui-btn secondary small" href={itemHref(g)}>Who holds it</a>
            <a class="ui-btn ghost small" href="/admin/entitlements?account={encodeURIComponent(g.accountId)}">Their graph</a>
          </div>
        </section>
      </div>
    {/if}
  {/snippet}
</Split>
{/if}

<style>
  .grp { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; }
  .grp.end { margin-left: auto; }
  .acc { width: 200px; max-width: 100%; }
  .which { min-width: 0; }
  .which select { min-width: 140px; max-width: 100%; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .rs { margin: 0; font-size: var(--t-small); overflow-wrap: anywhere; }
  @media (max-width: 760px) { .grp, .acc { width: 100%; } .grp.end { margin-left: 0; } .which { flex: 1; } }
</style>
