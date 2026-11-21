<script lang="ts">
  // The admin overview: what waits (open tickets and the shop's numbers, a figure each), then every
  // section your roles open, grouped as in the rail, one line each. Both lists come from
  // lib/admin/shell/sections, so the overview and the rail always agree.
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import Icon from '$lib/components/Icon.svelte';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { can, session } from '$lib/tool/session.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { fail } from '$lib/tool/notices.svelte';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { visibleSections, type Section } from '$lib/admin/shell/sections';

  let counts = $state<AdminCountsQuery | null>(null);
  const review = can('emotes.review');
  const reports = review || can('emotes.takedown') || can('reports.manage');

  $effect(() => {
    gql(ADMIN_COUNTS, { review, reports })
      .then((r) => (counts = r))
      .catch(fail);
  });

  // The catalog's size for the cosmetics rows, and your own look for the welcome line.
  const cosmetics = can('cosmetics.manage') || can('cosmetics.grant');
  let catalog = $state<{ paints: number; badges: number } | null>(null);
  $effect(() => {
    if (cosmetics) loadCatalog().then((c) => (catalog = { paints: c.paints.size, badges: c.badges.size })).catch(() => {});
    if (session.me) loadOwned(session.me.id, true).catch(() => {});
  });
  $effect(() => {
    loadShopCounts();
  });
  const me = $derived(session.me);
  const look = $derived((me && looks[me.id]) || NO_LOOK);

  const groups = $derived.by(() => {
    void session.me;
    return visibleSections(can);
  });
  /** The number a section shows: open tickets, the shop's counts, the catalog's size. */
  function figure(s: Section): number | undefined {
    switch (s.href) {
      case '/admin/review': return counts?.reviewQueue?.total;
      case '/admin/reports': return counts?.reportQueue?.total;
      case '/admin/shop/items': return shopCounts.items;
      case '/admin/shop/orders': return shopCounts.orders;
      case '/admin/shop/codes': return shopCounts.codes;
      case '/admin/cosmetics/paints': return catalog?.paints;
      case '/admin/cosmetics/badges': return catalog?.badges;
    }
    return undefined;
  }
  // What waits: the ticket queues you work, in big figures at the top.
  const waiting = $derived(groups.flatMap((g) => g.items).filter((s) => s.urgent));
  const loaded = $derived(counts !== null);
  const globalSet = can('global_set.edit');
</script>

<svelte:head><title>Admin - cyex.tv</title></svelte:head>

<header class="head">
  <h1>Welcome back{#if me}, <PaintedName name={me.identities[0]?.displayName ?? ''} paint={look.paint} />{/if}</h1>
  <p>What waits for you, and every section your roles open.</p>
</header>

{#if waiting.length}
  <section class="waits" aria-label="Waiting">
    {#each waiting as s (s.href)}
      {@const n = figure(s)}
      <a class="wait" href={s.href} class:some={(n ?? 0) > 0}>
        <span class="adm-label"><Icon name={s.icon} size={14} />{s.label}</span>
        <span class="fig" aria-hidden={!loaded}>{#if loaded}{count(n ?? 0)}{:else}<span class="ph ui-skeleton"></span>{/if}</span>
        <span class="sub">{#if !loaded}&nbsp;{:else if (n ?? 0) > 0}{s.href === '/admin/review' ? 'waiting for a verdict' : 'open, oldest first'}{:else}Nothing waiting{/if}</span>
      </a>
    {/each}
  </section>
{/if}

<div class="index">
  {#each groups as g (g.label)}
    <section class="grp" aria-labelledby="g-{g.label}">
      <h2 class="adm-label" id="g-{g.label}"><Icon name={g.icon} size={14} />{g.label}</h2>
      <ul class="adm-rows">
        {#each g.items as s (s.href)}
          {@const n = figure(s)}
          <li>
            <a class="row" href={s.href}>
              <Icon name={s.icon} />
              <span class="what"><strong>{s.label}</strong><span class="text">{s.text}</span></span>
              {#if n !== undefined}<span class="n" aria-hidden="true">{count(n)}</span><span class="sr-only">, {count(n)}</span>{/if}
              <Icon name="chevron-right" size={14} />
            </a>
          </li>
        {/each}
        {#if g.label === 'System' && globalSet}
          <li>
            <a class="row" href={counts ? `/me/sets/${counts.globalSet.id}` : '/admin'} aria-disabled={!counts}>
              <Icon name="layers" />
              <span class="what"><strong>Global set</strong><span class="text">Shown in every channel that uses cyex.tv.</span></span>
              <Icon name="chevron-right" size={14} />
            </a>
          </li>
        {/if}
      </ul>
    </section>
  {/each}
</div>

<style>
  .head { margin-bottom: 20px; }
  .head h1 { margin: 0; font-size: var(--t-page); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; overflow-wrap: anywhere; }
  .head p { margin: 8px 0 0; color: var(--muted); font-size: var(--t-ui); }

  /* What waits: a status line of figures (cyex-app), one cell per queue, hairlines between. */
  .waits { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(220px, 100%), 1fr)); margin-bottom: 24px;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); overflow: hidden; }
  .wait { display: grid; gap: 4px; padding: 14px 16px; color: var(--text); text-decoration: none; box-shadow: -1px 0 0 var(--border);
    transition: background-color var(--fast); }
  .wait .adm-label { display: flex; align-items: center; gap: 6px; }
  .fig { font-size: 28px; font-weight: 600; line-height: 1.15; letter-spacing: -.02em; font-variant-numeric: tabular-nums; color: var(--muted); }
  .wait.some .fig { color: var(--text); }
  .wait.some .adm-label :global(svg) { color: var(--accent-text); }
  .ph { display: block; width: 48px; height: 28px; border-radius: var(--radius); }
  .sub { font-size: var(--t-fine); color: var(--dim); }
  .wait:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }

  /* Every section, grouped: a caption and a panel of one-line rows. */
  .index { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(380px, 100%), 1fr)); gap: 20px 16px; align-items: start; }
  .grp { display: grid; gap: 8px; min-width: 0; }
  .grp h2 { display: flex; align-items: center; gap: 6px; padding: 0 2px; }
  .row { display: flex; align-items: center; gap: 12px; min-height: 56px; padding: 8px 12px; color: var(--text); text-decoration: none;
    transition: background-color var(--fast); }
  .row > :global(svg:first-child) { flex: none; color: var(--dim); }
  .row > :global(svg:last-child) { flex: none; color: var(--dim); }
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-weight: 600; font-size: var(--t-small); }
  .text { font-size: var(--t-fine); color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { flex: none; font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .row:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .row:active, .wait:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) {
    .row:hover, .wait:hover { background: var(--row-hover); }
    .row:hover > :global(svg:first-child) { color: var(--muted); }
  }
  @media (max-width: 520px) { .text { white-space: normal; } }
</style>
