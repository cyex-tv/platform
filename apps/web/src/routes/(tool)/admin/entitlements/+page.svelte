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
  import PageHead from '$lib/tool/PageHead.svelte';
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
</script>

<svelte:head><title>Entitlement graph - Admin - cyex.tv</title></svelte:head>

<PageHead title="Entitlement graph" line="How each badge, paint and role was given: by staff, an order, a code, or by being an account at all." />

{#if !allowed}
  <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
{:else}
  <div class="adm-toolbar" role="search">
    <form class="grp" onsubmit={openAccount}>
      <label class="adm-find acc">
        <Icon name="user" size={15} />
        <span class="sr-only">An account, by ID or its admin page link</span>
        <input class="ui-input ui-mono" bind:value={accountInput} placeholder="Account ID or admin link" autocomplete="off" spellcheck="false" />
      </label>
      <button class="ui-btn secondary" type="submit" disabled={!looksLikeId(accountInput.trim().split('/').filter(Boolean).pop() ?? '')}>Open</button>
      <a class="ui-btn ghost" href="/admin/users">Find by name</a>
    </form>
    <form class="grp" onsubmit={openItem}>
      <Segmented label="Kind" size="s" compact options={[{ value: 'BADGE', label: 'Badge' }, { value: 'PAINT', label: 'Paint' }, ...(options.ROLE.length ? [{ value: 'ROLE' as const, label: 'Role' }] : [])]} value={kind} onchange={(v) => (kind = v)} />
      <label class="which">
        <span class="sr-only">Which {kind.toLowerCase()}</span>
        <select class="ui-input" bind:value={item} disabled={!list.length}>
          {#each list as o (o.id)}<option value={o.id}>{o.name}</option>{:else}<option value="">None yet</option>{/each}
        </select>
      </label>
      <button class="ui-btn secondary" type="submit" disabled={!item}>Show holders</button>
    </form>
  </div>

  {#if target}
    <LoadGate source={L} label="Drawing the graph…">
      {#snippet children(d)}
        {#if d}
          <section class="ui-panel graph" aria-label="Entitlement graph">
            <header class="adm-ip-head">
              <h2><bdi>{d.root}</bdi></h2>
              {#if d.accountId}<a class="ui-btn ghost small" href="/admin/users/{encodeURIComponent(d.accountId)}">Account page</a>{/if}
            </header>
            <div class="pad"><EntitlementTree grants={d.grants} root={d.root} by={d.by} /></div>
          </section>
        {/if}
      {/snippet}
    </LoadGate>
  {:else}
    <State title="Pick an account or an item.">Every account page links here too.</State>
  {/if}
{/if}

<style>
  .grp { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; }
  .grp + .grp { margin-left: auto; }
  .acc { width: 200px; max-width: 100%; }
  .which { min-width: 0; }
  .which select { min-width: 140px; max-width: 100%; }
  .graph { overflow: hidden; }
  .graph .adm-ip-head { position: static; }
  .pad { padding: 12px 16px 16px; }
  @media (max-width: 760px) { .grp, .acc { width: 100%; } .grp + .grp { margin-left: 0; } .which { flex: 1; } }
</style>
