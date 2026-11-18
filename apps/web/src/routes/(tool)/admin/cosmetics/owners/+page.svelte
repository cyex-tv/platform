<script lang="ts">
  // Who owns what, across every badge and paint (cosmeticEntitlements): narrow by kind, by one
  // cosmetic, or by one account (its ID, or the login it signed in with). Revoke from here.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import { looksLikeId } from '$lib/admin/audit';
  import Avatar from '$lib/components/Avatar.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, date, iso } from '$lib/format';
  import type { CosmeticKind, EntitlementsQuery, Platform } from '$lib/gql/graphql';
  import { ENTITLEMENTS, REVOKE_COSMETIC, USER } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';

  type Row = EntitlementsQuery['cosmeticEntitlements']['items'][number];
  const PER = 25;
  const q = (k: string) => page.url.searchParams.get(k) ?? '';
  const kindQ = $derived<CosmeticKind>(q('kind') === 'paint' ? 'PAINT' : 'BADGE');

  // The form mirrors the address; Search writes it back.
  let cosmetic = $state(q('cosmetic'));
  let who = $state(q('account'));
  let target = $state<Row | null>(null);
  let dialog = $state<ReasonDialog>();
  const grant = can('cosmetics.grant');

  /** An account the field names: an ID as is, otherwise a login on Twitch, then Kick. */
  async function resolve(text: string, fetch: typeof globalThis.fetch): Promise<string | null | undefined> {
    const t = text.trim().replace(/^@/, '');
    if (!t) return undefined;
    if (looksLikeId(t)) return t.toUpperCase();
    for (const p of ['TWITCH', 'KICK'] as Platform[]) {
      const r = await gql(USER, { platform: p, login: t.toLowerCase() }, fetch);
      if (r.channelByLogin) return r.channelByLogin.account.id;
    }
    return null;
  }

  const L = loader(async ({ fetch }) => {
    const catalog = await loadCatalog();
    const kind = q('kind') === 'paint' ? 'PAINT' : 'BADGE';
    const accountId = await resolve(q('account'), fetch);
    const pageNo = Math.max(1, Number(q('page')) || 1);
    if (accountId === null) return { catalog, kind, accountId, rows: [] as Row[], total: 0, pageNo, nobody: true };
    const r = (await gql(ENTITLEMENTS, { kind, accountId: accountId ?? null, cosmeticId: q('cosmetic') || null, page: pageNo, perPage: PER }, fetch)).cosmeticEntitlements;
    return { catalog, kind, accountId, rows: r.items, total: r.total, pageNo, nobody: false };
  });

  const choices = $derived.by(() => {
    const c = L.data?.catalog;
    if (!c) return [];
    return [...(kindQ === 'BADGE' ? c.badges.values() : c.paints.values())].sort((a, b) => a.name.localeCompare(b.name));
  });

  function go(next: Record<string, string>) {
    const p = new URLSearchParams();
    const all = { kind: kindQ === 'PAINT' ? 'paint' : '', cosmetic, account: who.trim(), page: '', ...next };
    for (const [k, v] of Object.entries(all)) if (v) p.set(k, v);
    goto(`?${p}`, { keepFocus: true });
  }

  function search(e: SubmitEvent) {
    e.preventDefault();
    go({});
  }

  async function revoke(reason: string) {
    const r = target;
    if (!r) return;
    try {
      await gql(REVOKE_COSMETIC, { accountId: r.account.id, kind: r.kind, cosmeticId: r.cosmeticId, reason: reason || null });
      notify(`Taken from ${iso(r.account.displayName)}.`);
      await L.reload();
    } catch (e) {
      fail(e);
      throw e;
    }
  }

  const filtered = $derived(!!(q('cosmetic') || q('account')));
</script>

<svelte:head><title>Owners - Admin - cyex.tv</title></svelte:head>

<PageHead title="Owners" line="Who has which badge or paint, and who gave it to them." />

<form class="find" onsubmit={search}>
  <Segmented label="Kind" options={[{ value: 'BADGE', label: 'Badges' }, { value: 'PAINT', label: 'Paints' }]} value={kindQ}
    onchange={(v) => { cosmetic = ''; go({ kind: v === 'PAINT' ? 'paint' : '', cosmetic: '' }); }} />
  <select class="ui-input" bind:value={cosmetic} aria-label={kindQ === 'BADGE' ? 'Badge' : 'Paint'}>
    <option value="">Any {kindQ === 'BADGE' ? 'badge' : 'paint'}</option>
    {#each choices as c (c.id)}<option value={c.id}>{iso(c.name)}</option>{/each}
  </select>
  <input class="ui-input" bind:value={who} placeholder="Account ID or login" aria-label="Account ID or login" autocomplete="off" spellcheck="false" />
  <button class="ui-btn secondary" type="submit">Search</button>
  {#if filtered}<a class="ui-btn ghost" href={kindQ === 'PAINT' ? '?kind=paint' : '?'}>Clear</a>{/if}
</form>

<LoadGate source={L} label="Looking up owners…">
{#snippet children(r)}
{#if r.nobody}
  <State title="Nobody found.">No account has that ID or has signed in with that login.</State>
{:else if r.rows.length === 0}
  <State title="No owners.">{filtered ? 'Nothing matches this search.' : 'Nobody owns one yet.'}</State>
{:else}
  {@const pages = Math.max(1, Math.ceil(r.total / PER))}
  <section class="ui-panel" aria-label="Owners">
    <ul class="ui-list">
      {#each r.rows as e (e.account.id + e.cosmeticId)}
        {@const badge = e.kind === 'BADGE' ? r.catalog.badges.get(e.cosmeticId) : undefined}
        {@const paint = e.kind === 'PAINT' ? r.catalog.paints.get(e.cosmeticId) : undefined}
        <li>
          <Avatar src={e.account.avatar} name={e.account.displayName} size={32} />
          <span class="who">
            <AccountLink id={e.account.id} name={e.account.displayName} />
            <span class="ui-hint ui-num">{date(e.grantedAt)}{#if e.grantedBy} · by <AccountLink id={e.grantedBy.id} name={e.grantedBy.displayName} />{/if}{#if e.reason} · <bdi>{clean(e.reason)}</bdi>{/if}</span>
          </span>
          <span class="what">
            {#if badge}<BadgeIcon {badge} size={22} />{/if}
            {#if paint}<PaintedName name={paint.name} {paint} />{:else}<bdi>{clean(badge?.name ?? 'Deleted')}</bdi>{/if}
          </span>
          {#if grant}<button class="ui-btn ghost small" type="button" onclick={() => { target = e; dialog?.open(); }}>Revoke</button>{/if}
        </li>
      {/each}
    </ul>
    {#if pages > 1}
      <nav class="paging" aria-label="Pages">
        <button class="ui-btn secondary small" type="button" disabled={r.pageNo <= 1} onclick={() => go({ page: r.pageNo > 2 ? String(r.pageNo - 1) : '' })}>Previous</button>
        <span class="ui-hint ui-num">{r.pageNo} / {pages} · {count(r.total)}</span>
        <button class="ui-btn secondary small" type="button" disabled={r.pageNo >= pages} onclick={() => go({ page: String(r.pageNo + 1) })}>Next</button>
      </nav>
    {:else}
      <p class="ui-hint total ui-num">{count(r.total)} {r.total === 1 ? 'entry' : 'entries'}</p>
    {/if}
  </section>
{/if}
{/snippet}
</LoadGate>

<ReasonDialog bind:this={dialog} title="Take it away?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  .find { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 16px; max-width: var(--measure); }
  .find .ui-input { flex: 1; min-width: min(180px, 100%); }
  .who { display: grid; flex: 1; min-width: 0; }
  .who :global(a) { font-weight: 600; color: var(--text); }
  .who .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what { display: inline-flex; align-items: center; gap: 8px; min-width: 0; max-width: 40%; }
  .paging { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 10px; border-top: 1px solid var(--border); }
  .total { margin: 0; padding: 10px 14px; border-top: 1px solid var(--border); }
  @media (max-width: 640px) {
    section :global(li) { flex-wrap: wrap; row-gap: 4px; }
    .what { order: 3; flex: 1 1 100%; max-width: 100%; padding-left: 44px; }
  }
</style>
