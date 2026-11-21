<script lang="ts">
  // Who holds (or held) one badge, paint or role, as a table: the account, how they got it, who
  // gave it and when, and when and by whom it was taken away. Held now or ever, a search over
  // names and IDs, sortable columns, 25 a page, and the whole filtered list as CSV. Rows come from
  // entitlementHolders (the same edges the entitlement graph draws).
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count, dateTime } from '$lib/format';
  import AccountLink from './AccountLink.svelte';
  import { holderRows, holdersCsv, how, type Grant, type HolderSort } from './graph';
  import { lookUp, who } from './names.svelte';

  type Props = {
    grants: Grant[];
    /** For the file name of the CSV. */
    itemName: string;
    /** Offered on rows held now, when set. */
    onrevoke?: (g: Grant) => void;
  };
  let { grants, itemName, onrevoke }: Props = $props();
  const PER = 25;

  let show = $state<'held' | 'ever'>('held');
  let sort = $state<HolderSort>('given');
  let desc = $state(true);
  let search = $state('');
  let pageNo = $state(1);

  // Names for the account column, the search and the CSV (looked up once each, in batches).
  $effect(() => {
    lookUp(grants.flatMap((g) => [g.accountId, g.grantedBy, g.revokedBy]));
  });
  const nameOf = (id: string) => {
    const w = who(id);
    return w ? w.name : undefined;
  };
  const rows = $derived(holderRows(grants, { show, sort, desc, search, nameOf }));
  const held = $derived(grants.filter((g) => g.active).length);
  const pages = $derived(Math.max(1, Math.ceil(rows.length / PER)));
  const shown = $derived(rows.slice((Math.min(pageNo, pages) - 1) * PER, Math.min(pageNo, pages) * PER));
  $effect(() => {
    void show;
    void search;
    void sort;
    void desc;
    pageNo = 1;
  });

  function by(col: HolderSort) {
    if (sort === col) desc = !desc;
    else {
      sort = col;
      desc = col === 'given' || col === 'ended';
    }
  }
  const aria = (col: HolderSort) => (sort === col ? (desc ? 'descending' : 'ascending') : 'none');

  function download() {
    const blob = new Blob([holdersCsv(rows, nameOf)], { type: 'text/csv;charset=utf-8' });
    const a = document.createElement('a');
    const slug = clean(itemName).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'item';
    a.href = URL.createObjectURL(blob);
    a.download = `holders-${slug}-${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  }
</script>

<div class="adm-toolbar">
  <Segmented label="Show" size="s" value={show} onchange={(v) => (show = v)}
    options={[{ value: 'held', label: `Held now (${count(held)})` }, { value: 'ever', label: `Ever (${count(grants.length)})` }]} />
  <label class="adm-find grow">
    <Icon name="search" size={15} />
    <span class="sr-only">Search holders by name or ID</span>
    <input class="ui-input" type="search" bind:value={search} placeholder="Name or account ID" autocomplete="off" spellcheck="false" />
  </label>
  <button class="ui-btn secondary small" type="button" disabled={!rows.length} onclick={download}><Icon name="download" size={15} />Export CSV</button>
</div>

{#if !rows.length}
  <State bare title={search ? 'Nobody matches.' : show === 'held' ? 'Nobody holds it now.' : 'Nobody has held it.'}>
    {#if show === 'held' && grants.length && !search}Switch to Ever to see who held it before.{/if}
  </State>
{:else}
  <div class="scroll">
    <table class="adm-table">
      <thead>
        <tr>
          <th aria-sort={aria('account')}><button type="button" class:on={sort === 'account'} onclick={() => by('account')}>Account{#if sort === 'account'}<Icon name={desc ? 'arrow-down' : 'arrow-up'} size={12} />{/if}</button></th>
          <th aria-sort={aria('how')}><button type="button" class:on={sort === 'how'} onclick={() => by('how')}>How{#if sort === 'how'}<Icon name={desc ? 'arrow-down' : 'arrow-up'} size={12} />{/if}</button></th>
          <th>Given by</th>
          <th aria-sort={aria('given')}><button type="button" class:on={sort === 'given'} onclick={() => by('given')}>Given{#if sort === 'given'}<Icon name={desc ? 'arrow-down' : 'arrow-up'} size={12} />{/if}</button></th>
          <th aria-sort={aria('ended')}><button type="button" class:on={sort === 'ended'} onclick={() => by('ended')}>Ended{#if sort === 'ended'}<Icon name={desc ? 'arrow-down' : 'arrow-up'} size={12} />{/if}</button></th>
          {#if onrevoke}<th class="shrink"><span class="sr-only">Actions</span></th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each shown as g (g.accountId + g.source + (g.grantedAt ?? '') + (g.revokedAt ?? ''))}
          <tr class:past={!g.active}>
            <td><span class="acc"><AccountLink id={g.accountId} /><a class="graph" href="/admin/entitlements?account={encodeURIComponent(g.accountId)}" title="Everything this account owns">Owns</a></span></td>
            <td>{how(g)}{#if g.reason}<span class="sub" title={g.reason}><bdi>{clean(g.reason)}</bdi></span>{/if}</td>
            <td>{#if g.grantedBy}<AccountLink id={g.grantedBy} />{:else}<span class="ui-hint">-</span>{/if}</td>
            <td class="when">{g.grantedAt ? dateTime(g.grantedAt) : '-'}</td>
            <td>{#if g.revokedAt}<span class="when">{dateTime(g.revokedAt)}</span>{#if g.revokedBy}<span class="sub">by <AccountLink id={g.revokedBy} /></span>{/if}{:else}<span class="ui-hint">Held</span>{/if}</td>
            {#if onrevoke}<td class="shrink act">{#if g.active && g.source !== 'DEFAULT' && g.source !== 'OWNER'}<button class="ui-btn ghost small" type="button" onclick={() => onrevoke(g)}>Revoke</button>{/if}</td>{/if}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <nav class="adm-pager" aria-label="Holder pages">
    {#if pages > 1}<button class="ui-btn secondary small" type="button" disabled={pageNo <= 1} onclick={() => (pageNo -= 1)}>Previous</button>{/if}
    <span>{pages > 1 ? `${count((pageNo - 1) * PER + 1)}-${count(Math.min(rows.length, pageNo * PER))} of ` : ''}{count(rows.length)} {rows.length === 1 ? 'holder' : 'holders'}</span>
    {#if pages > 1}<button class="ui-btn secondary small" type="button" disabled={pageNo >= pages} onclick={() => (pageNo += 1)}>Next</button>{/if}
  </nav>
{/if}

<style>
  .scroll { overflow-x: auto; }
  .adm-table { min-width: 720px; }
  th button { display: inline-flex; align-items: center; gap: 4px; min-height: 28px; padding: 0; border: 0; background: none; font: inherit; letter-spacing: inherit; text-transform: inherit; color: inherit; cursor: pointer; }
  th button.on { color: var(--text); }
  th button.on :global(svg) { color: var(--accent-text, var(--accent)); }
  th button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; border-radius: var(--radius); }
  tr.past td { color: var(--muted); }
  .acc { display: flex; align-items: baseline; gap: 8px; min-width: 150px; }
  .acc :global(a:first-child) { font-weight: 600; color: var(--text); }
  .graph { font-size: var(--t-fine); color: var(--dim); }
  .sub { display: block; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); color: var(--dim); }
  .when { white-space: nowrap; font-size: var(--t-fine); }
  td:nth-child(2) { min-width: 180px; }
  .act { text-align: right; }
</style>
