<script lang="ts">
  // The owners lookup: pick or search a badge, paint or role on the left; on the right, everyone
  // who holds it now (or ever did), how they got it, who gave it and when, sortable and as CSV.
  // The other way round (one account: everything it owns) is the entitlement graph; the account
  // field here opens it. Rows come from entitlementHolders, as the graph's do.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fetchWith, gql } from '$lib/api';
  import UserFilter from '$lib/components/UserFilter.svelte';
  import { pastedAccount, topHolders } from '$lib/admin/people';
  import HoldersTable from '$lib/admin/HoldersTable.svelte';
  import type { Grant } from '$lib/admin/graph';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import type { GrantKind } from '$lib/gql/graphql';
  import { ALL_ROLES, ENTITLEMENT_HOLDERS, REVOKE_COSMETIC } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import { gridKeys } from '$lib/admin/ui/pick';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can, session } from '$lib/tool/session.svelte';

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

  // The other way round: an account to everything it owns (UserFilter: focused empty, the 5
  // accounts holding the most; typing, the people search; an ID or admin link pasted works).
  const openAccount = (id: string) => id && goto(`/admin/entitlements?account=${encodeURIComponent(id)}`);

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
  const sample = $derived(session.me?.identities[0]?.displayName ?? 'cyex.tv');
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !itemId || e.defaultPrevented || document.querySelector('dialog[open]')) return;
    if ((e.target as HTMLElement | null)?.closest('input, textarea, select')) return;
    e.preventDefault();
    go({ item: '' });
  }
  const KINDS = $derived([
    { value: 'badge' as Kind, label: 'Badges', icon: 'badge' as const },
    { value: 'paint' as Kind, label: 'Paints', icon: 'palette' as const },
    ...(rolesToo ? [{ value: 'role' as Kind, label: 'Roles', icon: 'shield' as const }] : []),
  ]);
</script>

<svelte:head><title>{current ? `${clean(current.name)} - Owners` : 'Owners'} - Admin - cyex.tv</title></svelte:head>

<svelte:window onkeydown={onkeydown} />

<Split label="Owners" open={allowed && !!itemId} onclose={() => go({ item: '' })}>
  <Bar title="Owners" count={allowed && C.data ? `${count(list.length)} ${kind}${list.length === 1 ? '' : 's'}` : null}>
    {#snippet tools()}
      {#if allowed}
        <Segmented label="Kind" size="s" compact options={KINDS} value={kind} onchange={(v) => { filter = ''; go({ kind: v === 'badge' ? '' : v, item: '' }); }} />
        <label class="adm-find find">
          <Icon name="search" size={15} />
          <span class="sr-only">Search {kind}s</span>
          <input class="ui-input" type="search" bind:value={filter} placeholder="Search {kind}s" autocomplete="off" spellcheck="false" />
        </label>
        {#if itemId}<label class="which">
          <span class="sr-only">Which {kind}</span>
          <select class="ui-input" value={current ? itemId : ''} disabled={!list.length} onchange={(e) => go({ item: e.currentTarget.value })}>
            {#if !current}<option value="" disabled>{C.data ? (list.length ? `Pick a ${kind}` : filter ? 'Nothing matches' : `No ${kind}s yet`) : 'Loading…'}</option>{/if}
            {#each list as x (x.id)}<option value={x.id}>{clean(x.name)}</option>{/each}
          </select>
        </label>{/if}
        <div class="who">
          <UserFilter label="An account: everything it owns" hideLabel placeholder="Account: everything it owns" width="240px" value=""
            onchange={openAccount} top={(signal) => topHolders(signal, fetchWith)} topLabel="Hold the most" ids={pastedAccount} />
        </div>
      {/if}
    {/snippet}
  </Bar>

  {#if !allowed}
    <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
  {:else}

    {#if !itemId}
      {#if C.data && !list.length}
        <State title={filter ? 'Nothing matches.' : `No ${kind}s yet.`} />
      {:else}
        <ul class="adm-sheet" style="--cell: 168px" use:gridKeys aria-label="{kind === 'role' ? 'Roles' : kind === 'paint' ? 'Paints' : 'Badges'}">
          {#each list as x, i (x.id)}
            <li>
              <button class="adm-cell tile" type="button" data-item={x.id} tabindex={i === 0 ? 0 : -1} onclick={() => go({ item: x.id })}>
                <span class="pic">
                  {#if x.badge}<BadgeIcon badge={x.badge} size={32} />{:else if x.paint}<PaintedName name={sample} paint={x.paint} />{:else}<Icon name="shield" size={22} />{/if}
                </span>
                <span class="nm"><bdi>{clean(x.name)}</bdi></span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <LoadGate source={H} label="Looking up holders…">
        {#snippet children(grants)}
          {#if grants}
            <HoldersTable {grants} itemName={current?.name ?? itemId} onrevoke={mayRevoke && kind !== 'role' ? (g) => { target = g; dialog?.open(); } : undefined} />
          {/if}
        {/snippet}
      </LoadGate>
    {/if}
  {/if}

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Owners</span></div>
    <p class="none">Pick a {kind} to see who holds it, how they got it and since when.</p>
  {/snippet}

  {#snippet panel()}
    {#if itemId}
      <div class="adm-ip-head">
        <span class="adm-label">{kind === 'role' ? 'Role' : kind === 'paint' ? 'Paint' : 'Badge'}</span>
        <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={() => go({ item: '' })} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name">
            {#if current?.badge}<BadgeIcon badge={current.badge} size={22} />{/if}
            {#if current?.paint}<PaintedName name={current.name} paint={current.paint} />{:else}<bdi>{clean(current?.name ?? H.data?.find((g) => g.itemName)?.itemName ?? itemId)}</bdi>{/if}
          </h2>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            <dt>Held now</dt><dd>{H.data ? count(H.data.filter((g) => g.active).length) : '…'}</dd>
            <dt>Ever</dt><dd>{H.data ? count(H.data.length) : '…'}</dd>
            <dt>ID</dt><dd class="mono">{itemId}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <div class="adm-acts row">
            <a class="ui-btn secondary small" href="/admin/entitlements?{kind}={encodeURIComponent(itemId)}"><Icon name="network" size={14} />As a graph</a>
          </div>
        </section>
      </div>
    {/if}
  {/snippet}
</Split>

<ReasonDialog bind:this={dialog} title="Take it away?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  :global(.adm-shell .adm-toolbar) .find { flex: 1 1 140px; max-width: 200px; }
  .which { flex: 1 1 180px; max-width: 240px; min-width: 0; }
  .which select { width: 100%; }
  .who { display: flex; gap: 8px; margin-left: auto; flex: 1 1 220px; min-width: 0; }
  :global(.adm-shell .adm-toolbar) .who .adm-find { flex: 1; max-width: none; }
  .tile { height: 100px; justify-items: center; text-align: center; }
  .tile .pic { height: 44px; max-width: 100%; overflow: hidden; font-size: 18px; font-weight: 700; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .adm-name { display: flex; align-items: center; gap: 10px; }
  @media (max-width: 719.98px) { :global(.adm-shell .adm-toolbar) .find, .which, .who { max-width: none; flex-basis: 100%; margin-left: 0; } }
</style>
