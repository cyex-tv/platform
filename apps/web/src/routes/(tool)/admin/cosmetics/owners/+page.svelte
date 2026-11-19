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

<PageHead title="Owners" line="Look up who holds a badge, paint or role, how they got it and since when." />

{#if !allowed}
  <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
{:else}
  <form class="ui-panel reverse" onsubmit={openAccount}>
    <label class="ui-field grow">
      <span class="ui-label">Or an account: everything it owns</span>
      <input class="ui-input" bind:value={who} oninput={() => (notFound = false)} placeholder="Login, account ID or its admin page link" autocomplete="off" spellcheck="false" />
    </label>
    <button class="ui-btn secondary" type="submit" disabled={!who.trim() || finding}>{finding ? 'Looking…' : 'Open its graph'}</button>
    {#if notFound}<p class="ui-hint miss" role="status">Nobody has signed in with that login.</p>{/if}
  </form>

  <div class="split">
    <section class="ui-panel items" aria-labelledby="items-h">
      <header><h2 id="items-h">Items</h2></header>
      <div class="pick">
        <Segmented label="Kind" size="s" compact options={KINDS} value={kind} onchange={(v) => { filter = ''; go({ kind: v === 'badge' ? '' : v, item: '' }); }} />
        <label class="find">
          <Icon name="search" size={15} />
          <span class="sr-only">Search {kind}s</span>
          <input class="ui-input" type="search" bind:value={filter} placeholder="Search {kind}s" autocomplete="off" spellcheck="false" />
        </label>
      </div>
      <LoadGate source={C} label="Loading items…">
        {#snippet children()}
          {#if !list.length}
            <p class="ui-hint empty">{filter ? 'Nothing matches.' : `No ${kind}s yet.`}</p>
          {:else}
            <ul class="choices" aria-label="{kind}s">
              {#each list as x (x.id)}
                <li>
                  <a href="?{new URLSearchParams({ ...(kind === 'badge' ? {} : { kind }), item: x.id })}" class:on={x.id === itemId} aria-current={x.id === itemId ? 'true' : undefined} data-sveltekit-keepfocus data-sveltekit-noscroll>
                    {#if x.badge}<BadgeIcon badge={x.badge} size={20} />{:else if !x.paint}<Icon name="shield" size={16} />{/if}
                    <span class="nm">{#if x.paint}<PaintedName name={x.name} paint={x.paint} />{:else}<bdi>{clean(x.name)}</bdi>{/if}</span>
                    {#if x.id === itemId}<span class="tick"><Icon name="check" size={15} /></span>{/if}
                  </a>
                </li>
              {/each}
            </ul>
          {/if}
        {/snippet}
      </LoadGate>
    </section>

    <section class="ui-panel holders" aria-labelledby="holders-h">
      <header>
        <h2 id="holders-h">{#if current || itemId}<bdi>{clean(current?.name ?? H.data?.find((g) => g.itemName)?.itemName ?? itemId)}</bdi>{:else}Holders{/if}</h2>
        {#if itemId}<a class="ui-btn ghost small" href="/admin/entitlements?{kind}={encodeURIComponent(itemId)}">As a graph</a>{/if}
      </header>
      {#if !itemId}
        <State bare title="Pick an item.">Its holders show here, with how and when they got it.</State>
      {:else}
        <LoadGate source={H} label="Looking up holders…">
          {#snippet children(grants)}
            {#if grants}
              <HoldersTable {grants} itemName={current?.name ?? itemId} onrevoke={mayRevoke && kind !== 'role' ? (g) => { target = g; dialog?.open(); } : undefined} />
            {/if}
          {/snippet}
        </LoadGate>
      {/if}
    </section>
  </div>
{/if}

<ReasonDialog bind:this={dialog} title="Take it away?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  .reverse { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 10px; padding: 12px; margin-bottom: 12px; }
  .grow { flex: 1; min-width: min(240px, 100%); }
  .miss { flex-basis: 100%; margin: 0; }
  .split { display: grid; min-width: 0; grid-template-columns: minmax(0, 1fr); gap: 12px; align-items: start; }
  @media (min-width: 1000px) { .split { grid-template-columns: minmax(240px, 300px) minmax(0, 1fr); } }
  .items header, .holders header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .holders h2 { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pick { display: grid; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  .find { position: relative; display: flex; align-items: center; }
  .find :global(svg) { position: absolute; left: 10px; color: var(--dim); pointer-events: none; }
  .find input { width: 100%; padding-left: 32px; }
  .empty { margin: 0; padding: 10px 14px; }
  .choices { list-style: none; margin: 0; padding: 4px 0; max-height: min(60vh, 640px); overflow-y: auto; }
  @media (max-width: 999px) { .choices { max-height: 240px; } }
  .choices a { display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 4px 14px; color: var(--muted); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .choices a:hover { color: var(--text); background: var(--surface-hover); } }
  .choices a:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .choices a.on { font-weight: 700; color: var(--text); }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tick { display: inline-flex; color: var(--accent-text, var(--accent)); }
</style>
