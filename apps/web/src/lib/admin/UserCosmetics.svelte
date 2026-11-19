<script lang="ts">
  // An account's badges and paints on the admin Users page (cosmetics.grant): what it owns (with
  // the reason and when), Give (from what it doesn't own, with a reason) and Revoke.
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import type { CosmeticKind, EntitlementsQuery } from '$lib/gql/graphql';
  import { clean, dateTime, iso } from '$lib/format';
  import { ENTITLEMENTS, REVOKE_COSMETIC } from '$lib/queries';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import GrantDialog from './GrantDialog.svelte';

  type Row = EntitlementsQuery['cosmeticEntitlements']['items'][number];
  let { account }: { account: { id: string; displayName: string } } = $props();

  let catalog = $state<Catalog | null>(null);
  let owned = $state<Record<CosmeticKind, Row[]>>({ BADGE: [], PAINT: [] });
  let problem = $state('');
  let loading = $state(true);
  let grantKind = $state<CosmeticKind>('BADGE');
  let grantDlg = $state<GrantDialog>();
  let revokeDlg = $state<ReasonDialog>();
  let target = $state<Row | null>(null);

  async function load() {
    loading = true;
    problem = '';
    try {
      const [cat, b, p] = await Promise.all([
        loadCatalog(),
        gql(ENTITLEMENTS, { kind: 'BADGE', accountId: account.id, page: 1, perPage: 100 }),
        gql(ENTITLEMENTS, { kind: 'PAINT', accountId: account.id, page: 1, perPage: 100 }),
      ]);
      catalog = cat;
      owned = { BADGE: b.cosmeticEntitlements.items, PAINT: p.cosmeticEntitlements.items };
    } catch (e) {
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      loading = false;
    }
  }
  $effect(() => {
    void account.id;
    load();
  });

  const nameOf = (kind: CosmeticKind, id: string) =>
    (kind === 'BADGE' ? catalog?.badges.get(id)?.name : catalog?.paints.get(id)?.name) ?? 'Deleted';
  const choices = (kind: CosmeticKind) => {
    const have = new Set(owned[kind].map((r) => r.cosmeticId));
    const all = kind === 'BADGE' ? [...(catalog?.badges.values() ?? [])] : [...(catalog?.paints.values() ?? [])];
    return all.filter((c) => !have.has(c.id)).map((c) => ({ id: c.id, name: c.name }));
  };

  function give(kind: CosmeticKind) {
    grantKind = kind;
    tick().then(() => grantDlg?.open());
  }

  async function revoke(reason: string) {
    const r = target;
    if (!r) return;
    try {
      await gql(REVOKE_COSMETIC, { accountId: account.id, kind: r.kind, cosmeticId: r.cosmeticId, reason: reason || null });
      notify(`${iso(nameOf(r.kind, r.cosmeticId))} taken away.`);
      await load();
    } catch (e) {
      fail(e);
      throw e;
    }
  }
</script>

<section class="ui-panel" aria-labelledby="uc-h">
  <header><h2 id="uc-h">Cosmetics</h2></header>
  {#if problem}
    <p class="ui-hint pad" role="alert">{problem} <button class="ui-btn ghost small" type="button" onclick={load}>Try again</button></p>
  {:else if loading && !catalog}
    <p class="ui-hint pad">Loading…</p>
  {:else}
    {#each ['BADGE', 'PAINT'] as const as kind (kind)}
      <div class="kind">
        <div class="kh">
          <h3>{kind === 'BADGE' ? 'Badges' : 'Paints'}</h3>
          <button class="ui-btn secondary small" type="button" onclick={() => give(kind)}>Give a {kind === 'BADGE' ? 'badge' : 'paint'}</button>
        </div>
        {#if owned[kind].length}
          <ul class="ui-list">
            {#each owned[kind] as r (r.cosmeticId)}
              {@const badge = kind === 'BADGE' ? catalog?.badges.get(r.cosmeticId) : undefined}
              {@const paint = kind === 'PAINT' ? catalog?.paints.get(r.cosmeticId) : undefined}
              <li>
                {#if badge}<BadgeIcon {badge} size={24} />{/if}
                <span class="what">
                  <strong>{#if paint}<PaintedName name={paint.name} {paint} />{:else}<bdi>{clean(nameOf(kind, r.cosmeticId))}</bdi>{/if}</strong>
                  <span class="ui-hint">{dateTime(r.grantedAt)}{#if r.grantedBy}{' · by '}<bdi>{clean(r.grantedBy.displayName)}</bdi>{/if}{#if r.reason}{' · '}<bdi>{clean(r.reason)}</bdi>{/if}</span>
                </span>
                <button class="ui-btn ghost small" type="button" onclick={() => { target = r; revokeDlg?.open(); }}>Revoke</button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="ui-hint pad">None.</p>
        {/if}
      </div>
    {/each}
  {/if}
</section>

<GrantDialog bind:this={grantDlg} kind={grantKind} {account} choices={choices(grantKind)} ongranted={load} />
<ReasonDialog bind:this={revokeDlg} title="Take it away?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  .kind + .kind { border-top: 1px solid var(--border); }
  .kh { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 10px 14px 4px; }
  .kh h3 { margin: 0; font-size: var(--t-small); font-weight: 600; color: var(--muted); }
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { display: flex; min-width: 0; font-weight: 600; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { margin: 0; padding: 6px 14px 12px; }
</style>
