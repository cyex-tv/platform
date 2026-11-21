<script lang="ts">
  // The owners lookup: pick or search a badge, paint or role on the left; on the right, everyone
  // who holds it now (or ever did), how they got it, who gave it and when, sortable and as CSV.
  // The other way round (one account: everything it owns) is the entitlement graph; the account
  // field here opens it. Rows come from entitlementHolders, as the graph's do.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import { looksLikeId } from '$lib/admin/audit';
  import HoldersTable from '$lib/admin/HoldersTable.svelte';
  import type { Grant } from '$lib/admin/graph';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog } from '$lib/cosmetics/store.svelte';
  import { clean, iso } from '$lib/format';
  import type { GrantKind, Platform } from '$lib/gql/graphql';
  import { ALL_ROLES, ENTITLEMENT_HOLDERS, REVOKE_COSMETIC, USER } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';

  type Kind = 'badge' | 'paint' | 'role';
  const allowed = can('entitlements.view') || can('cosmetics.grant');
  const mayRevoke = can('cosmetics.grant');
  const rolesToo = can('roles.manage') || can('roles.assign');

  const params = $derived(page.url.searchParams);
  const kind = $derived<Kind>(params.get('kind') === 'paint' ? 'paint' : params.get('kind') === 'role' && rolesToo ? 'role' : 'badge');
  const itemId = $derived(params.get('item') ?? '');

  // Everything that can be held: the catalog, and the roles when you may see them.
  const C = loader(async () => {
    const [cat, roles] = await Promise.all([loadCatalog(), rolesToo ? gql(ALL_ROLES, {}).then((r) => r.roles).catch(() => []) : Promise.resolve([])]);
    return { cat, roles: roles.filter((r) => !r.builtIn || r.builtIn === 'ADMIN') };
  });
  let filter = $state('');
  const list = $derived.by(() => {
    const d = C.data;
    if (!d) return [];
    const all =
      kind === 'badge'
        ? [...d.cat.badges.values()].map((b) => ({ id: b.id, name: b.name, badge: b, paint: undefined }))
        : kind === 'paint'
          ? [...d.cat.paints.values()].map((p) => ({ id: p.id, name: p.name, badge: undefined, paint: p }))
          : d.roles.map((r) => ({ id: r.id, name: r.name, badge: undefined, paint: undefined }));
    const q = filter.trim().toLowerCase();
    return all.filter((x) => !q || x.name.toLowerCase().includes(q) || x.id.toLowerCase() === q).sort((a, b) => a.name.localeCompare(b.name));
  });
  const current = $derived(list.find((x) => x.id === itemId) ?? null);

  function go(next: Record<string, string>) {
    const q = new URLSearchParams();
    const all = { kind: kind === 'badge' ? '' : kind, item: itemId, ...next };
    for (const [k, v] of Object.entries(all)) if (v) q.set(k, v);
    goto(`?${q}`, { keepFocus: true, noScroll: true });
  }

  const H = loader(async ({ fetch }) => {
    const id = itemId;
    if (!id || !allowed) return null;
    const r = await gql(ENTITLEMENT_HOLDERS, { kind: kind.toUpperCase() as GrantKind, id }, fetch);
    return r.entitlementHolders as Grant[];
  });

  // The other way round: an account (its ID or a login) to everything it owns.
  let who = $state('');
  let finding = $state(false);
  let notFound = $state(false);
  async function openAccount(e: SubmitEvent) {
    e.preventDefault();
    const t = who.trim().replace(/^@/, '').split('/').filter(Boolean).pop() ?? '';
    if (!t) return;
    notFound = false;
    if (looksLikeId(t)) return goto(`/admin/entitlements?account=${t.toUpperCase()}`);
    finding = true;
    try {
      for (const p of ['TWITCH', 'KICK'] as Platform[]) {
        const r = await gql(USER, { platform: p, login: t.toLowerCase() });
        if (r.channelByLogin) return goto(`/admin/entitlements?account=${r.channelByLogin.account.id}`);
      }
      notFound = true;
    } catch (err) {
      fail(err);
    } finally {
      finding = false;
    }
  }

  let target = $state<Grant | null>(null);
  let dialog = $state<ReasonDialog>();
  async function revoke(reason: string) {
    const g = target;
    if (!g || g.kind === 'ROLE') return;
    try {
      await gql(REVOKE_COSMETIC, { accountId: g.accountId, kind: g.kind, cosmeticId: g.itemId, reason: reason || null });
      notify(`${iso(current?.name ?? g.itemName)} taken away.`);
      await H.reload();
    } catch (err) {
      fail(err);
      throw err;
    }
  }
  const KINDS = $derived([
    { value: 'badge' as Kind, label: 'Badges', icon: 'badge' as const },
    { value: 'paint' as Kind, label: 'Paints', icon: 'palette' as const },
    ...(rolesToo ? [{ value: 'role' as Kind, label: 'Roles', icon: 'shield' as const }] : []),
  ]);
</script>

<svelte:head><title>Owners - Admin - cyex.tv</title></svelte:head>

<PageHead title="Owners" line="Who holds a badge, paint or role, how they got it and since when." />

{#if !allowed}
  <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
{:else}
  <div class="adm-toolbar" role="search">
    <Segmented label="Kind" size="s" compact options={KINDS} value={kind} onchange={(v) => { filter = ''; go({ kind: v === 'badge' ? '' : v, item: '' }); }} />
    <label class="adm-find find">
      <Icon name="search" size={15} />
      <span class="sr-only">Search {kind}s</span>
      <input class="ui-input" type="search" bind:value={filter} placeholder="Search {kind}s" autocomplete="off" spellcheck="false" />
    </label>
    <label class="which">
      <span class="sr-only">Which {kind}</span>
      <select class="ui-input" value={current ? itemId : ''} disabled={!list.length} onchange={(e) => go({ item: e.currentTarget.value })}>
        {#if !current}<option value="" disabled>{C.data ? (list.length ? `Pick a ${kind}` : filter ? 'Nothing matches' : `No ${kind}s yet`) : 'Loading…'}</option>{/if}
        {#each list as x (x.id)}<option value={x.id}>{clean(x.name)}</option>{/each}
      </select>
    </label>
    <form class="who" onsubmit={openAccount}>
      <label class="adm-find">
        <Icon name="user" size={15} />
        <span class="sr-only">An account: everything it owns</span>
        <input class="ui-input" bind:value={who} oninput={() => (notFound = false)} placeholder="Account login or ID" autocomplete="off" spellcheck="false" />
      </label>
      <button class="ui-btn secondary" type="submit" disabled={!who.trim() || finding}>{finding ? 'Looking…' : 'Open its graph'}</button>
    </form>
  </div>
  {#if notFound}<p class="ui-hint miss" role="status">Nobody has signed in with that login.</p>{/if}

  {#if !itemId}
    <State title="Pick an item.">Its holders show here, with how and when they got it.</State>
  {:else}
    <div class="ihead">
      <h2 class="adm-name">
        {#if current?.badge}<BadgeIcon badge={current.badge} size={22} />{/if}
        {#if current?.paint}<PaintedName name={current.name} paint={current.paint} />{:else}<bdi>{clean(current?.name ?? H.data?.find((g) => g.itemName)?.itemName ?? itemId)}</bdi>{/if}
      </h2>
      <a class="ui-btn ghost small" href="/admin/entitlements?{kind}={encodeURIComponent(itemId)}"><Icon name="network" size={14} />As a graph</a>
    </div>
    <LoadGate source={H} label="Looking up holders…">
      {#snippet children(grants)}
        {#if grants}
          <HoldersTable {grants} itemName={current?.name ?? itemId} onrevoke={mayRevoke && kind !== 'role' ? (g) => { target = g; dialog?.open(); } : undefined} />
        {/if}
      {/snippet}
    </LoadGate>
  {/if}
{/if}

<ReasonDialog bind:this={dialog} title="Take it away?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  .find { flex: 1 1 160px; max-width: 220px; }
  .which { flex: 1 1 180px; max-width: 240px; min-width: 0; }
  .which select { width: 100%; }
  .who { display: flex; gap: 8px; margin-left: auto; flex: 0 1 320px; min-width: 0; }
  .who .adm-find { flex: 1; }
  .miss { margin: -8px 0 12px; }
  .ihead { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 12px; }
  .ihead h2 { display: flex; align-items: center; gap: 10px; min-width: 0; }
  @media (max-width: 760px) { .find, .which, .who { max-width: none; flex-basis: 100%; margin-left: 0; } }
</style>
