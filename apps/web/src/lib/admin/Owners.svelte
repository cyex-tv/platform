<script lang="ts">
  // Who owns a badge or paint (cosmeticEntitlements, newest first, 20 a page), each with the
  // reason and who gave it, and Revoke (an optional reason for the audit log).
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import type { CosmeticKind, EntitlementsQuery } from '$lib/gql/graphql';
  import { clean, count, date, iso } from '$lib/format';
  import { ENTITLEMENTS, REVOKE_COSMETIC } from '$lib/queries';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Row = EntitlementsQuery['cosmeticEntitlements']['items'][number];
  type Props = { kind: CosmeticKind; cosmeticId: string; name: string; onchange?: (total: number) => void };
  let { kind, cosmeticId, name, onchange }: Props = $props();
  const PER = 20;

  let rows = $state<Row[]>([]);
  let total = $state(0);
  let pageNo = $state(1);
  let busy = $state(false);
  let problem = $state('');
  let target = $state<Row | null>(null);
  let dlg = $state<ReasonDialog>();

  export async function load(p = pageNo) {
    busy = true;
    problem = '';
    try {
      const r = await gql(ENTITLEMENTS, { kind, cosmeticId, page: p, perPage: PER });
      rows = r.cosmeticEntitlements.items;
      total = r.cosmeticEntitlements.total;
      pageNo = p;
      onchange?.(total);
    } catch (e) {
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      busy = false;
    }
  }
  $effect(() => {
    void cosmeticId;
    load(1);
  });

  async function revoke(reason: string) {
    const r = target;
    if (!r) return;
    try {
      await gql(REVOKE_COSMETIC, { accountId: r.account.id, kind, cosmeticId, reason: reason || null });
      notify(`${iso(name)} taken from ${iso(r.account.displayName)}.`);
      await load(rows.length === 1 && pageNo > 1 ? pageNo - 1 : pageNo);
    } catch (e) {
      fail(e);
      throw e;
    }
  }
  const pages = $derived(Math.max(1, Math.ceil(total / PER)));
</script>

<div class="owners" aria-busy={busy}>
  {#if problem}
    <p class="ui-hint pad" role="alert">{problem} <button class="ui-btn ghost small" type="button" onclick={() => load()}>Try again</button></p>
  {:else if !rows.length}
    <p class="ui-hint pad">{busy ? 'Loading…' : 'Nobody owns it yet.'}</p>
  {:else}
    <ul class="ui-list">
      {#each rows as r (r.account.id)}
        <li>
          <Avatar src={r.account.avatar} name={r.account.displayName} size={28} />
          <span class="what">
            <a href="/users/{encodeURIComponent(r.account.id)}"><bdi>{clean(r.account.displayName)}</bdi></a>
            <span class="ui-hint">{date(r.grantedAt)}{#if r.grantedBy} · by <bdi>{clean(r.grantedBy.displayName)}</bdi>{/if}{#if r.reason} · <bdi>{clean(r.reason)}</bdi>{/if}</span>
          </span>
          <button class="ui-btn ghost small" type="button" onclick={() => { target = r; dlg?.open(); }}>Revoke</button>
        </li>
      {/each}
    </ul>
    {#if pages > 1}
      <nav class="paging" aria-label="Owner pages">
        <button class="ui-btn secondary small" type="button" disabled={pageNo <= 1 || busy} onclick={() => load(pageNo - 1)}>Previous</button>
        <span class="ui-hint ui-num">{pageNo} / {pages} · {count(total)}</span>
        <button class="ui-btn secondary small" type="button" disabled={pageNo >= pages || busy} onclick={() => load(pageNo + 1)}>Next</button>
      </nav>
    {/if}
  {/if}
</div>

<ReasonDialog bind:this={dlg} title="Revoke {iso(name)}?" hint="If they show it, it stops at once. The reason goes in the audit log." action="Revoke" danger optional onsubmit={revoke} />

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { font-weight: 600; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { margin: 0; padding: 10px 14px; }
  .paging { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 10px; }
</style>
