<script lang="ts">
  // The entitlement graph as flush hairline rows: the root (an account, or a badge, paint or
  // role), one group per source (a staff member, an order, a code batch, every account, the owner
  // list; or, for an item, one per holder), and under each the grants that came from it, held or
  // taken away, with the date in mono. A grant is a button: picking it opens the page's inspector.
  // The same grants sit in a flush table behind the List view. The show and view controls live in
  // the page's toolbar and come in as props.
  import { gridKeys } from '$lib/admin/ui/pick';
  import State from '$lib/components/State.svelte';
  import { date } from '$lib/format';
  import AccountLink from './AccountLink.svelte';
  import { branches, KIND_LABEL, SOURCE_LABEL, type Grant } from './graph';

  let {
    grants,
    root,
    by = 'source',
    show = 'all',
    view = 'tree',
    picked = null,
    onpick,
  }: {
    grants: Grant[];
    /** The root node's text: the account's name, or the item's. */
    root: string;
    /** 'source' for an account (where each thing came from), 'account' for an item (who holds it). */
    by?: 'source' | 'account';
    show?: 'all' | 'held';
    view?: 'tree' | 'list';
    /** The picked grant: its index in `grants`. */
    picked?: number | null;
    onpick?: (index: number | null) => void;
  } = $props();

  const groups = $derived(branches(grants, by, show));
  const SOURCE = SOURCE_LABEL;
  const shown = $derived(show === 'held' ? grants.filter((g) => g.active) : grants);
  const at = (g: Grant) => grants.indexOf(g);
  const itemHref = (g: Grant) =>
    g.kind === 'ROLE' ? `/admin/entitlements?role=${encodeURIComponent(g.itemId)}` : `/admin/entitlements?${g.kind.toLowerCase()}=${encodeURIComponent(g.itemId)}`;
  const first = $derived(picked !== null && shown.includes(grants[picked]) ? picked : at(shown[0]));
  const when = (g: Grant) => (g.grantedAt ? date(g.grantedAt) : g.source === 'DEFAULT' || g.source === 'OWNER' ? 'Always' : 'Before the log');
</script>

{#if shown.length === 0}
  <State bare title={show === 'held' ? 'Nothing held now.' : 'Nothing given yet.'} />
{:else if view === 'tree'}
  <div class="tree" aria-label="How each was given" use:gridKeys={'button[data-item]'}>
    <p class="root"><strong><bdi>{root}</bdi></strong></p>
    {#each groups as b (b.key)}
      <section class="grp">
        <h3 class="gh adm-label">
          <span>{by === 'account' ? 'Holder' : SOURCE[b.source]}</span>
          <span class="gn">{#if b.staffId}<AccountLink id={b.staffId} />{:else if b.href}<a href={b.href}>{b.label}</a>{:else}{b.label}{/if}</span>
          <span class="n adm-num">{b.grants.length}</span>
        </h3>
        <ul>
          {#each b.grants as g (at(g))}
            {@const i = at(g)}
            <li class:gone={!g.active}>
              <button type="button" class="leaf" data-item={i} aria-current={picked === i ? 'true' : undefined} tabindex={i === first ? 0 : -1} onclick={() => onpick?.(i)}>
                <span class="kind adm-flags"><span>{KIND_LABEL[g.kind]}</span></span>
                <span class="what">
                  {#if by === 'source'}<bdi>{g.itemName || 'Deleted'}</bdi>{:else}<span>{SOURCE[g.source]}{#if g.orderName} · {g.orderName}{/if}{#if g.codeBatchName} · {g.codeBatchName}{/if}</span>{/if}
                </span>
                {#if !g.active}<span class="adm-flags"><span class="warn">Taken away</span></span>{/if}
                <span class="dt adm-num">{g.revokedAt ? date(g.revokedAt) : when(g)}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
{:else}
  <table class="adm-table stack">
    <thead><tr><th>What</th><th>{by === 'account' ? 'Holder' : 'Source'}</th><th>Given</th><th>By</th><th>Taken away</th></tr></thead>
    <tbody>
      {#each shown as g (at(g))}
        <tr class:gone={!g.active} data-row aria-selected={picked === at(g)} tabindex={at(g) === first ? 0 : -1} onclick={() => onpick?.(at(g))}
          onkeydown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget) { e.preventDefault(); onpick?.(at(g)); } }}>
          <td class="pk" data-l="What"><span class="lead"><span class="ui-hint">{KIND_LABEL[g.kind]}</span> <bdi>{g.itemName || 'Deleted'}</bdi></span></td>
          <td data-l={by === 'account' ? 'Holder' : 'Source'}>{#if by === 'account'}<AccountLink id={g.accountId} />{:else}{SOURCE[g.source]}{#if g.orderName} · {g.orderName}{/if}{#if g.codeBatchName} · {g.codeBatchName}{/if}{/if}</td>
          <td class="dim mono" data-l="Given">{g.grantedAt ? date(g.grantedAt) : '-'}</td>
          <td data-l="By">{#if g.grantedBy}<AccountLink id={g.grantedBy} />{:else}-{/if}</td>
          <td class="mono" data-l="Taken away">{g.revokedAt ? date(g.revokedAt) : '-'}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  .tree { border-top: 1px solid var(--border); }
  .root { margin: 0; display: flex; align-items: center; min-height: 44px; border-bottom: 1px solid var(--border); font-size: var(--t-ui); }
  .gh { display: flex; align-items: center; gap: 10px; min-height: 36px; margin: 0; border-bottom: 1px solid var(--border); }
  .gn { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-small); font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--text); }
  .gh .n { margin-left: auto; }
  .grp ul { margin-left: 8px; border-left: 1px solid var(--border-strong); }
  .grp li { border-bottom: 1px solid var(--border); }
  .leaf { display: grid; grid-template-columns: 64px minmax(0, 1fr) auto auto; align-items: center; gap: 12px; width: 100%; min-height: 40px; padding: 0 0 0 14px; border: 0; background: none;
    font: inherit; font-size: var(--t-small); color: var(--text); text-align: left; cursor: pointer; }
  .leaf:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .leaf[aria-current='true'] .what { font-weight: 700; }
  .leaf[aria-current='true'] .kind { color: var(--accent-text); }
  .what { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .dt { color: var(--muted); font-size: var(--t-fine); }
  .gone .what { color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .leaf:hover { background: var(--row-hover); } }
  .gone td { color: var(--muted); }
  .stack { table-layout: auto; }
</style>
