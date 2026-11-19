<script lang="ts">
  // The entitlement graph, drawn as a tree: the root (an account, or a badge, paint or role),
  // one branch per source (a staff member, an order, a code batch, every account, the owner
  // list; or, for an item, one per holder), and the leaves: what came from it, held or taken away.
  // A list of the same grants sits behind the List switch, for reading line by line.
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { dateTime } from '$lib/format';
  import AccountLink from './AccountLink.svelte';
  import { branches, KIND_LABEL, SOURCE_LABEL, tally, type Grant } from './graph';

  let {
    grants,
    root,
    by = 'source',
  }: {
    grants: Grant[];
    /** The root node's text: the account's name, or the item's. */
    root: string;
    /** 'source' for an account (where each thing came from), 'account' for an item (who holds it). */
    by?: 'source' | 'account';
  } = $props();

  let view = $state<'tree' | 'list'>('tree');
  let show = $state<'all' | 'held'>('all');
  const groups = $derived(branches(grants, by, show));
  const t = $derived(tally(grants));
  const SOURCE = SOURCE_LABEL;
  const shown = $derived(show === 'held' ? grants.filter((g) => g.active) : grants);
  const itemHref = (g: Grant) =>
    g.kind === 'ROLE' ? `/admin/entitlements?role=${encodeURIComponent(g.itemId)}` : `/admin/entitlements?${g.kind.toLowerCase()}=${encodeURIComponent(g.itemId)}`;
</script>

<div class="bar">
  <p class="sum ui-hint">
    {#if by === 'source'}
      Holds <span class="ui-num">{t.roles}</span> {t.roles === 1 ? 'role' : 'roles'}, <span class="ui-num">{t.badges}</span> {t.badges === 1 ? 'badge' : 'badges'} and <span class="ui-num">{t.paints}</span> {t.paints === 1 ? 'paint' : 'paints'}{#if t.gone}; <span class="ui-num">{t.gone}</span> taken away{/if}.
    {:else}
      <span class="ui-num">{grants.filter((g) => g.active).length}</span> hold it now{#if t.gone}; <span class="ui-num">{t.gone}</span> lost it{/if}.
    {/if}
  </p>
  <div class="controls">
    <Segmented label="Show" size="s" options={[{ value: 'all', label: 'Everything' }, { value: 'held', label: 'Held now' }]} value={show} onchange={(v) => (show = v)} />
    <Segmented label="View" size="s" options={[{ value: 'tree', label: 'Graph', icon: 'network' }, { value: 'list', label: 'List', icon: 'list' }]} value={view} onchange={(v) => (view = v)} />
  </div>
</div>

{#if shown.length === 0}
  <State bare title={show === 'held' ? 'Nothing held now.' : 'Nothing given yet.'} />
{:else if view === 'tree'}
  <div class="tree" role="tree" aria-label="How each was given">
    <div class="root" role="treeitem" aria-selected="false" aria-expanded="true">
      <span class="node root-node"><strong><bdi>{root}</bdi></strong></span>
      <ul role="group">
        {#each groups as b (b.key)}
          <li role="treeitem" aria-selected="false" aria-expanded="true">
            <span class="node src">
              <span class="ui-label">{by === 'account' ? 'Holder' : SOURCE[b.source]}</span>
              {#if b.staffId}<AccountLink id={b.staffId} />{:else if b.href}<a href={b.href}>{b.label}</a>{:else}<span>{b.label}</span>{/if}
            </span>
            <ul role="group">
              {#each b.grants as g, i (g.kind + g.itemId + g.accountId + i)}
                <li role="treeitem" aria-selected="false" class:gone={!g.active}>
                  <span class="node leaf">
                    <span class="what">
                      <span class="ui-chip">{KIND_LABEL[g.kind]}</span>
                      {#if by === 'source'}<a href={itemHref(g)}><bdi>{g.itemName || 'Deleted'}</bdi></a>{:else}<span class="src-line">{SOURCE[g.source]}{#if g.orderName} · {g.orderName}{/if}{#if g.codeBatchName} · {g.codeBatchName}{/if}</span>{/if}
                      {#if !g.active}<span class="ui-chip rejected">Taken away</span>{/if}
                    </span>
                    <span class="when ui-hint">
                      {#if g.grantedAt}Given <span class="ui-num">{dateTime(g.grantedAt)}</span>{#if by === 'account' && g.grantedBy}{' by '}<AccountLink id={g.grantedBy} />{/if}{:else if g.source === 'DEFAULT' || g.source === 'OWNER'}Always{:else}Given before the log{/if}
                      {#if g.revokedAt}{' · '}taken away <span class="ui-num">{dateTime(g.revokedAt)}</span>{#if g.revokedBy}{' by '}<AccountLink id={g.revokedBy} />{/if}{#if g.revokeReason}{` (${g.revokeReason})`}{/if}{/if}
                    </span>
                    {#if g.reason}<span class="reason ui-hint">“{g.reason}”</span>{/if}
                  </span>
                </li>
              {/each}
            </ul>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{:else}
  <div class="table-wrap">
    <table class="ui-table">
      <thead><tr><th>What</th><th>{by === 'account' ? 'Holder' : 'Source'}</th><th>Given</th><th>By</th><th>Taken away</th></tr></thead>
      <tbody>
        {#each shown as g, i (g.kind + g.itemId + g.accountId + i)}
          <tr class:gone={!g.active}>
            <td><span class="ui-chip">{KIND_LABEL[g.kind]}</span> <bdi>{g.itemName || 'Deleted'}</bdi></td>
            <td>{#if by === 'account'}<AccountLink id={g.accountId} />{:else}{SOURCE[g.source]}{#if g.orderName} · {g.orderName}{/if}{#if g.codeBatchName} · {g.codeBatchName}{/if}{/if}</td>
            <td class="ui-num">{g.grantedAt ? dateTime(g.grantedAt) : '–'}</td>
            <td>{#if g.grantedBy}<AccountLink id={g.grantedBy} />{:else}–{/if}</td>
            <td>{#if g.revokedAt}<span class="ui-num">{dateTime(g.revokedAt)}</span>{#if g.revokedBy}{' · '}<AccountLink id={g.revokedBy} />{/if}{#if g.revokeReason}<span class="ui-hint">{` ${g.revokeReason}`}</span>{/if}{:else}–{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px 16px; margin-bottom: 12px; }
  .sum { margin: 0; }
  .controls { display: flex; flex-wrap: wrap; gap: 12px; }
  ul { list-style: none; margin: 0; padding: 0; }
  /* The tree: hairline connectors from each node to its parent. */
  .tree { overflow-x: auto; }
  .tree ul { margin-left: 14px; padding-left: 18px; border-left: 1px solid var(--border-strong); }
  .tree li { position: relative; padding-top: 8px; }
  .tree li::before { content: ''; position: absolute; left: -18px; top: 26px; width: 16px; border-top: 1px solid var(--border-strong); }
  .tree li:last-child::after { content: ''; position: absolute; left: -19px; top: 27px; bottom: 0; width: 2px; background: var(--bg); }
  .node { display: inline-grid; gap: 2px; min-width: 0; max-width: 100%; padding: 8px 12px; border: 1px solid var(--border);
    border-radius: var(--radius-card); background: var(--surface); }
  .root-node { padding: 10px 14px; }
  .src { grid-auto-flow: row; }
  .src .ui-label { line-height: 1.4; }
  .leaf .what { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: var(--text-sub); }
  .when, .reason { overflow-wrap: anywhere; }
  .gone .node { background: transparent; border-style: dashed; }
  .gone .what a, .gone .what .src-line { color: var(--muted); }
  .table-wrap { overflow-x: auto; }
  .gone td { color: var(--muted); }
</style>
