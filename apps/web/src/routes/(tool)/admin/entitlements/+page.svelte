<script lang="ts">
  // The entitlement graph: for an account, where each badge, paint and role it holds or held came
  // from (shop products and their orders, code batches, events, staff members, the built-in
  // roles); for a badge, paint or role, who holds it and how. Drawn as a graph you can pan, zoom
  // and drag (GraphCanvas), with a legend that hides and shows kinds, "Find in graph" over its
  // nodes and the catalogue, and the picked node's facts in the inspector (?node= in the
  // address). The List view shows the same grants as rows, for reading and assistive tech.
  // Open it from an account page, or pick an account (UserFilter) or an item (the search) here.
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { fetchWith, gql } from '$lib/api';
  import { BP } from '$lib/breakpoints';
  import UserFilter from '$lib/components/UserFilter.svelte';
  import { topHolders, pastedAccount } from '$lib/admin/people';
  import type { GrantKind } from '$lib/gql/graphql';
  import { ACCOUNT, ALL_ROLES, COSMETICS, ENTITLEMENT_GRAPH, ENTITLEMENT_HOLDERS } from '$lib/queries';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import EntitlementTree from '$lib/admin/EntitlementTree.svelte';
  import GraphCanvas from '$lib/admin/GraphCanvas.svelte';
  import GraphNodePanel from '$lib/admin/GraphNodePanel.svelte';
  import { tally, type Grant } from '$lib/admin/graph';
  import { buildGraph, findCatalog, findNodes, kindCounts, NODE_KINDS, NODE_LABEL, visible, type CatalogItem, type GNode, type NodeKind } from '$lib/admin/entitlement-graph';
  import { lookUp, who } from '$lib/admin/names.svelte';
  import { clean, count } from '$lib/format';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { pickInUrl } from '$lib/admin/ui/pick';
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

  // The catalogue, for "Find in graph" to open an item's holders.
  let catalog = $state<CatalogItem[]>([]);
  $effect(() => {
    if (!allowed) return;
    gql(COSMETICS, {})
      .then((r) => (catalog = [...catalog.filter((c) => c.kind === 'ROLE'), ...r.cosmetics.badges.map((b) => ({ kind: 'BADGE' as const, id: b.id, name: b.name })), ...r.cosmetics.paints.map((p) => ({ kind: 'PAINT' as const, id: p.id, name: p.name }))]))
      .catch(() => {});
    if (can('roles.manage') || can('roles.assign'))
      gql(ALL_ROLES, {})
        .then((r) => (catalog = [...catalog.filter((c) => c.kind !== 'ROLE'), ...r.roles.filter((x) => !x.builtIn || x.builtIn === 'ADMIN').map((x) => ({ kind: 'ROLE' as const, id: x.id, name: x.name }))]))
        .catch(() => {});
  });

  const openAccount = (id: string) => goto(id ? `?account=${encodeURIComponent(id)}` : '?');

  const L = loader(async ({ fetch }) => {
    const t = target;
    if (!t || !allowed) return null;
    if (t.kind === 'ACCOUNT') {
      const [g, a] = await Promise.all([gql(ENTITLEMENT_GRAPH, { accountId: t.id }, fetch), gql(ACCOUNT, { id: t.id }, fetch)]);
      return { by: 'source' as const, kind: t.kind, id: t.id, name: clean(a.account?.displayName) || t.id, grants: g.entitlementGraph as Grant[], accountId: t.id };
    }
    const g = await gql(ENTITLEMENT_HOLDERS, { kind: t.kind as GrantKind, id: t.id }, fetch);
    const grants = g.entitlementHolders as Grant[];
    const name = grants.find((x) => x.itemName)?.itemName || catalog.find((o) => o.kind === t.kind && o.id === t.id)?.name || t.id;
    return { by: 'account' as const, kind: t.kind, id: t.id, name, grants, accountId: null };
  });

  // Show everything or what is held now; the graph or the list; the layout and node size.
  let show = $state<'all' | 'held'>('all');
  let view = $state<'graph' | 'tree' | 'list'>('graph');
  let layout = $state<'force' | 'radial'>('force');
  let nodeSize = $state(4);
  let hidden = $state<ReadonlySet<NodeKind>>(new Set());
  const graph = $derived(L.data ? buildGraph(L.data.grants, { kind: L.data.kind, id: L.data.id, name: L.data.name }, show) : null);
  const counts = $derived(graph ? kindCounts(graph.nodes) : null);
  const shown = $derived(graph ? visible(graph, hidden) : null);
  const sum = $derived(L.data ? tally(L.data.grants) : null);

  // Names of the accounts in the graph (staff members, holders), looked up once each.
  $effect(() => {
    if (graph) lookUp(graph.nodes.map((n) => (n.root ? null : n.accountId)));
  });
  const names = $derived(graph ? graph.nodes.map((n) => (n.accountId && !n.root ? (who(n.accountId)?.name ?? '') : '')).join('\u0000') : '');
  const labelOf = (n: GNode) => (n.name || (n.accountId ? clean(who(n.accountId)?.name) || n.accountId.slice(-8) : ''));

  // The picked node (?node=), a node id; an old link's grant number picks that grant's leaf.
  const nodeParam = $derived(params.get('node'));
  const leafOf = (g: Grant, by: 'source' | 'account') => (by === 'account' ? `account:${g.accountId}` : `${g.kind.toLowerCase()}:${g.itemId}`);
  const selected = $derived.by(() => {
    if (!nodeParam || !graph || !L.data) return null;
    if (graph.byId.has(nodeParam)) return nodeParam;
    if (/^\d{1,6}$/.test(nodeParam)) {
      const g = L.data.grants[Number(nodeParam)];
      if (g && graph.byId.has(leafOf(g, L.data.by))) return leafOf(g, L.data.by);
    }
    return null;
  });
  const picked = $derived(selected && graph ? graph.byId.get(selected) ?? null : null);
  const pickNode = (id: string | null) => void pickInUrl('node', id);
  // The list views pick grants: the grant's leaf is the node.
  const pickedGrant = $derived(picked && picked.grants.length && !picked.root ? picked.grants[0] : null);
  const pickGrant = (i: number | null) => pickNode(i === null || !L.data ? null : leafOf(L.data.grants[i], L.data.by));

  // Hiding a kind leaves out its nodes (the root stays); hiding the picked one drops the pick.
  function toggleKind(k: NodeKind) {
    const next = new Set(hidden);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    hidden = next;
    if (picked && !picked.root && next.has(picked.kind)) pickNode(null);
  }

  // Open from the graph: an item's holders, or an account's own graph.
  function openNode(n: GNode) {
    if (n.itemId && (n.kind === 'badge' || n.kind === 'paint' || n.kind === 'role')) goto(`?${n.kind}=${encodeURIComponent(n.itemId)}`);
    else if (n.accountId) goto(`?account=${encodeURIComponent(n.accountId)}`);
  }

  // ---- Find in graph ----
  let canvasApi = $state<ReturnType<typeof GraphCanvas> | null>(null);
  let q = $state('');
  let open = $state(false);
  let active = $state(0);
  const hits = $derived(shown && q.trim() ? findNodes(shown.nodes, q, labelOf) : []);
  const matchSet = $derived(q.trim() ? new Set(hits.map((n) => n.id)) : null);
  const catHits = $derived(findCatalog(catalog, q, 5));
  type Opt = { key: string; node?: GNode; item?: CatalogItem };
  const opts = $derived<Opt[]>([...hits.slice(0, 8).map((n) => ({ key: `n:${n.id}`, node: n })), ...catHits.map((it) => ({ key: `c:${it.kind}:${it.id}`, item: it }))]);
  $effect(() => {
    void q;
    active = 0;
  });
  function choose(o: Opt | undefined) {
    if (!o) return;
    if (o.node) {
      if (view !== 'graph') view = 'graph';
      pickNode(o.node.id);
      canvasApi?.centre(o.node.id);
    } else if (o.item) {
      q = '';
      goto(`?${o.item.kind.toLowerCase()}=${encodeURIComponent(o.item.id)}`);
    }
    open = false;
  }
  function onfindkey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      open = true;
      if (opts.length) active = (active + (e.key === 'ArrowDown' ? 1 : opts.length - 1)) % opts.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(opts[active]);
    } else if (e.key === 'Escape') {
      if (open && q) {
        e.preventDefault();
        open = false;
      } else if (q) {
        e.preventDefault();
        q = '';
      }
    }
  }
  const optLabel = (o: Opt) => (o.node ? `${NODE_LABEL[o.node.kind]}: ${labelOf(o.node) || 'Unknown'}` : `${o.item!.kind === 'ROLE' ? 'Role' : o.item!.kind === 'BADGE' ? 'Badge' : 'Paint'}: ${o.item!.name}`);

  const description = $derived(
    graph && shown && counts
      ? `${count(shown.nodes.length)} nodes and ${count(shown.edges.length)} links. ` +
          NODE_KINDS.filter((k) => counts[k]).map((k) => `${counts[k]} ${NODE_LABEL[k].toLowerCase()}${hidden.has(k) ? ' (hidden)' : ''}`).join(', ') + '.'
      : '',
  );
  // The graph runs down to the window's bottom edge (whatever wraps above it), at least 380px.
  function fillDown(node: HTMLElement) {
    const fit = () => {
      const top = node.getBoundingClientRect().top + window.scrollY;
      node.style.height = `${Math.max(window.innerWidth < BP.phone ? 560 : 380, Math.round(window.innerHeight - top - 20))}px`;
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(document.body);
    window.addEventListener('resize', fit);
    return { destroy: () => { ro.disconnect(); window.removeEventListener('resize', fit); } };
  }
  const rootLabel = $derived(L.data ? (L.data.by === 'source' ? L.data.name : `${L.data.kind === 'ROLE' ? 'Role' : L.data.kind === 'BADGE' ? 'Badge' : 'Paint'}: ${L.data.name}`) : '');
</script>

<svelte:head><title>Entitlement graph - Admin - cyex.tv</title></svelte:head>

{#if !allowed}
  <Bar title="Entitlement graph" />
  <State level={2} title="No access here.">This needs See where things came from (entitlements.view) or Give cosmetics.</State>
{:else}
<Split label={picked ? NODE_LABEL[picked.kind] : 'Node'} open={!!picked} onclose={() => pickNode(null)}>
  <Bar title="Entitlement graph" count={shown ? `${count(shown.nodes.length)} nodes · ${count(shown.edges.length)} links` : null}>
    {#snippet tools()}
      <div class="grp acc">
        <UserFilter label="Account" hideLabel placeholder="Account: name, login or ID" width="240px" value={target?.kind === 'ACCOUNT' ? target.id : ''}
          onchange={openAccount} top={(signal) => topHolders(signal, fetchWith)} topLabel="Hold the most" ids={pastedAccount} />
      </div>
      <div class="grp find" role="search">
        <label class="field">
          <Icon name="search" size={15} />
          <span class="sr-only">Find in graph, or a badge, paint or role</span>
          <input class="ui-input" type="search" placeholder={graph ? 'Find in graph' : 'Find a badge, paint or role'} autocomplete="off" spellcheck="false"
            role="combobox" aria-expanded={open && !!q.trim()} aria-controls="find-list" aria-autocomplete="list"
            aria-activedescendant={open && opts[active] ? `find-${active}` : undefined}
            bind:value={q} oninput={() => (open = true)} onfocus={() => (open = true)} onblur={() => setTimeout(() => (open = false), 120)} onkeydown={onfindkey} />
          {#if q.trim() && graph}<span class="hits adm-num" aria-live="polite">{hits.length === 1 ? '1 match' : `${count(hits.length)} matches`}</span>{/if}
        </label>
        {#if open && q.trim()}
          <ul class="pop" id="find-list" role="listbox" aria-label="Matches">
            {#if graph}<li class="head adm-label" role="presentation">In this graph{hits.length > 8 ? ` · first 8 of ${count(hits.length)}` : ''}</li>{/if}
            {#each opts as o, i (o.key)}
              {#if o.item && (i === 0 || opts[i - 1].node)}<li class="head adm-label" role="presentation">Open who holds it</li>{/if}
              <li id="find-{i}" role="option" aria-selected={i === active} class:on={i === active}
                onpointerdown={(e) => { e.preventDefault(); choose(o); }} onpointerenter={() => (active = i)}>
                <span class="dot" style:--c="var(--node-{o.node ? o.node.kind : o.item!.kind.toLowerCase()})"></span>
                <bdi>{optLabel(o)}</bdi>
                {#if o.item}<Icon name="arrow-right" size={13} />{/if}
              </li>
            {:else}
              <li class="none" role="presentation">Nothing matches.</li>
            {/each}
          </ul>
        {/if}
      </div>
      {#if target}
        <div class="grp end">
          <Segmented label="Show" size="s" options={[{ value: 'all', label: 'Everything' }, { value: 'held', label: 'Held now' }]} value={show} onchange={(v) => (show = v)} />
          <Segmented label="View" size="s" compact options={[{ value: 'graph', label: 'Graph', icon: 'network' }, { value: 'tree', label: 'Groups', icon: 'layers' }, { value: 'list', label: 'List', icon: 'list' }]} value={view} onchange={(v) => (view = v)} />
        </div>
      {/if}
    {/snippet}
  </Bar>

  {#if target}
    <LoadGate source={L} label="Drawing the graph…">
      {#snippet children(d)}
        {#if d && graph && shown && counts}
          {#if view === 'graph'}
            <div class="stage" use:fillDown>
              <div class="strip">
                <p class="who"><span class="dot" style:--c="var(--node-{graph.byId.get(graph.root)?.kind})"></span><strong><bdi>{rootLabel}</bdi></strong></p>
                <div class="ctl">
                  <Segmented label="Layout" size="s" options={[{ value: 'force', label: 'Force' }, { value: 'radial', label: 'Radial' }]} value={layout} onchange={(v) => (layout = v)} />
                  <label class="size"><span class="sl">Node size</span>
                    <input class="ui-input" type="number" min="1" max="10" step="1" value={nodeSize}
                      oninput={(e) => { const v = Number(e.currentTarget.value); if (v >= 1 && v <= 10) nodeSize = Math.round(v); }} />
                  </label>
                  <span class="zoom">
                    <IconButton label="Zoom out" icon="minus" size="s" onclick={() => canvasApi?.zoom(1 / 1.4)} />
                    <IconButton label="Zoom in" icon="plus" size="s" onclick={() => canvasApi?.zoom(1.4)} />
                    <button type="button" class="ui-btn ghost small" onclick={() => canvasApi?.fit()}>Fit</button>
                  </span>
                </div>
              </div>
              <div class="ground">
                {#if shown.nodes.length > 1}
                  <GraphCanvas bind:this={canvasApi} {graph} {hidden} {selected} matches={matchSet} {layout} size={nodeSize} {labelOf} {names}
                    label="Entitlement graph of {rootLabel}" {description} onselect={pickNode} onopen={openNode} />
                {:else}
                  <State bare title={show === 'held' ? 'Nothing held now.' : 'Nothing given yet.'} />
                {/if}
              </div>
              <ul class="legend" aria-label="Kinds: pick one to hide or show it">
                {#each NODE_KINDS.filter((k) => counts[k]) as k (k)}
                  <li>
                    <button type="button" aria-pressed={!hidden.has(k)} disabled={k === graph.byId.get(graph.root)?.kind && counts[k] === 1} onclick={() => toggleKind(k)}>
                      <span class="dot" class:off={hidden.has(k)} style:--c="var(--node-{k})"></span>
                      <span class="k">{NODE_LABEL[k]}</span><span class="adm-num">{count(counts[k])}</span>
                    </button>
                  </li>
                {/each}
                <li class="key"><span class="dot ring"></span>Taken away</li>
              </ul>
            </div>
          {:else}
            <EntitlementTree grants={d.grants} root={rootLabel} by={d.by} {show} view={view === 'tree' ? 'tree' : 'list'} picked={pickedGrant} onpick={pickGrant} />
          {/if}
        {/if}
      {/snippet}
    </LoadGate>
  {:else}
    <State bare title="Pick an account, or find a badge, paint or role.">Every account page links here too.</State>
  {/if}

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Graph</span></div>
    {#if L.data && sum && counts}
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi>{rootLabel}</bdi></h2>
          {#if L.data.accountId}<a class="ui-btn ghost small" href="/admin/users/{encodeURIComponent(L.data.accountId)}">Account page</a>{/if}
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Held now</h3>
          <dl class="adm-kv">
            {#if L.data.by === 'source'}
              <dt>Roles</dt><dd>{sum.roles}</dd><dt>Badges</dt><dd>{sum.badges}</dd><dt>Paints</dt><dd>{sum.paints}</dd>
            {:else}
              <dt>Holders</dt><dd>{L.data.grants.length - sum.gone}</dd>
            {/if}
            <dt>Taken away</dt><dd>{sum.gone}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">In the graph</h3>
          <dl class="adm-kv">
            {#each NODE_KINDS.filter((k) => counts[k]) as k (k)}<dt>{NODE_LABEL[k]}</dt><dd>{count(counts[k])}</dd>{/each}
          </dl>
          <p class="none">Pick a node to see how it was given. Drag to move, scroll to zoom, double-click an item for its holders.</p>
        </section>
      </div>
    {:else}
      <p class="none">Pick an account or an item, then a node to see who gave it and when.</p>
    {/if}
  {/snippet}

  {#snippet panel()}
    {#if picked && graph && L.data}
      <GraphNodePanel node={picked} {graph} grants={L.data.grants} {labelOf} onclose={() => pickNode(null)} />
    {/if}
  {/snippet}
</Split>
{/if}

<style>
  .grp { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; }
  .grp.end { margin-left: auto; }
  .acc { max-width: 100%; }
  .find { position: relative; }
  .field { position: relative; display: flex; align-items: center; color: var(--dim); }
  .field :global(svg) { position: absolute; left: 10px; pointer-events: none; }
  .field input { width: 260px; max-width: 100%; padding-left: 32px; padding-right: 84px; }
  .hits { position: absolute; right: 10px; font-size: var(--t-fine); color: var(--dim); pointer-events: none; }
  .pop { position: absolute; top: calc(100% + 4px); left: 0; z-index: var(--z-pop); width: max(100%, 320px); max-height: 380px; overflow: auto; margin: 0; padding: 4px 0;
    list-style: none; background: var(--elevated); border: 1px solid var(--border-strong); border-radius: var(--radius-card); box-shadow: var(--shadow-pop); }
  .pop .head { padding: 8px 12px 4px; }
  .pop [role='option'] { display: flex; align-items: center; gap: 8px; min-height: 34px; padding: 0 12px; font-size: var(--t-small); cursor: pointer; }
  .pop [role='option'] bdi { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pop [role='option'].on { background: var(--row-hover); font-weight: 600; }
  .pop [role='option'] :global(svg) { color: var(--dim); }
  .pop [role='option'].on :global(svg) { color: var(--accent-text); }
  .pop .none { padding: 8px 12px; font-size: var(--t-small); color: var(--dim); }
  .dot { flex: none; display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: var(--c); }
  .dot.off { background: transparent; box-shadow: inset 0 0 0 1.5px var(--c); opacity: .6; }
  .dot.ring { background: transparent; box-shadow: inset 0 0 0 2.5px var(--dim); }

  /* The graph: the page ground, hairline frame, a slim strip above, the legend below. */
  .stage { display: grid; grid-template-rows: auto minmax(0, 1fr) auto; height: 560px; border: 1px solid var(--border); background: var(--bg); }
  .strip { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px 16px; min-height: 44px; padding: 4px 8px 4px 12px; border-bottom: 1px solid var(--border); }
  .who { display: flex; align-items: center; gap: 8px; margin: 0; min-width: 0; font-size: var(--t-small); }
  .who bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ctl { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 14px; }
  .size { display: flex; align-items: center; gap: 8px; font-size: var(--t-small); color: var(--muted); }
  .size input { width: 64px; min-height: 30px; font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .zoom { display: flex; align-items: center; gap: 2px; }
  .ground { position: relative; min-height: 0; overflow: hidden; }
  .ground :global(.state) { height: 100%; }
  .legend { display: flex; flex-wrap: wrap; align-items: center; gap: 0 4px; margin: 0; padding: 4px 8px; list-style: none; border-top: 1px solid var(--border); }
  .legend button { display: inline-flex; align-items: center; gap: 7px; min-height: 32px; padding: 0 8px; border: 0; background: none; border-radius: var(--radius); font: inherit; font-size: var(--t-small); color: var(--text); cursor: pointer; }
  .legend button[aria-pressed='false'] .k { color: var(--dim); text-decoration: line-through; }
  .legend button:disabled { cursor: default; }
  .legend .adm-num { color: var(--dim); font-size: var(--t-fine); }
  .legend .key { display: inline-flex; align-items: center; gap: 7px; margin-left: auto; padding: 0 8px; font-size: var(--t-fine); color: var(--dim); }
  @media (hover: hover) and (pointer: fine) { .legend button:not(:disabled):hover { background: var(--row-hover); } }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .adm-sec .none { padding: 12px 0 0; }
  @media (max-width: 719.98px) {
    .grp, .acc, .find, .field, .field input { width: 100%; }
    .grp.end { margin-left: 0; }
    .field input { font-size: 16px; }
    .stage { margin-inline: calc(-1 * var(--page-pad)); border-inline: 0; }
    .strip { padding-inline: var(--page-pad); }
    .sl { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
    .size input { width: 56px; }
    .ctl { gap: 6px 10px; }
    .legend { flex-wrap: nowrap; overflow-x: auto; padding-inline: var(--page-pad); }
    .legend .key { margin-left: 0; white-space: nowrap; }
    .legend button { white-space: nowrap; min-height: 44px; }
  }
</style>
