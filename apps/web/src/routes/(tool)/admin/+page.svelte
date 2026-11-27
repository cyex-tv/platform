<script lang="ts">
  // The admin overview, a working dashboard: what waits (the review queue and open reports, each
  // with its number and the next few as small tiles that open them), recent staff activity (the
  // audit log's latest rows), and in the panel on the right the shop today (orders and revenue)
  // and quick actions. Every part asks only for what your roles allow and fails on its own. The
  // sections index at the foot comes from lib/admin/shell/sections, so it always agrees with the rail.
  import { gql } from '$lib/api';
  import { clean, clock, count, date, dateTime } from '$lib/format';
  import Icon from '$lib/components/Icon.svelte';
  import type { AdminCountsQuery, AdminOrdersQuery, AuditQuery, ReportQueueQuery, ReviewQueueQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS, ADMIN_ORDERS, AUDIT, REPORT_QUEUE, REVIEW_QUEUE } from '$lib/queries';
  import { pictureFor, ready, size } from '$lib/emote';
  import { display } from '$lib/dpr.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { fail } from '$lib/tool/notices.svelte';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { price } from '$lib/shop/shop';
  import { visibleSections, type Section } from '$lib/admin/shell/sections';
  import { actionText } from '$lib/admin/audit';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { adminCountVars, reviewWaiting } from '$lib/admin/review-count';
  import { quiet } from '$lib/quiet';

  const review = can('emotes.review');
  const reports = review || can('emotes.takedown') || can('reports.manage');
  const auditOk = can('audit.view');
  const ordersOk = can('shop.orders');
  const cosmetics = can('cosmetics.manage') || can('cosmetics.grant');
  const globalSet = can('global_set.edit');

  let counts = $state<AdminCountsQuery | null>(null);
  let nextReview = $state<ReviewQueueQuery['reviewQueue']['items'] | null>(null);
  let nextReports = $state<ReportQueueQuery['reportQueue']['items'] | null>(null);
  let recent = $state<AuditQuery['auditLog'] | null>(null);
  let orders = $state<AdminOrdersQuery['orders']['items'] | null>(null);
  let catalog = $state<{ paints: number; badges: number } | null>(null);
  const NEXT = 6;

  $effect(() => {
    gql(ADMIN_COUNTS, adminCountVars(can))
      .then((r) => (counts = r))
      .catch(fail);
    if (review) void quiet(gql(REVIEW_QUEUE, { page: 1, perPage: NEXT, filter: null }).then((r) => (nextReview = r.reviewQueue.items)), 'overview review');
    if (reports) void quiet(gql(REPORT_QUEUE, { page: 1, perPage: 40 }).then((r) => (nextReports = r.reportQueue.items)), 'overview reports');
    if (auditOk) void quiet(gql(AUDIT, { limit: 8 }).then((r) => (recent = r.auditLog)), 'overview audit');
    if (ordersOk) void quiet(gql(ADMIN_ORDERS, { page: 1, perPage: 100 }).then((r) => (orders = r.orders.items)), 'overview orders');
    if (cosmetics) loadCatalog().then((c) => (catalog = { paints: c.paints.size, badges: c.badges.size })).catch(() => {});
    if (session.me) loadOwned(session.me.id, true).catch(() => {});
    loadShopCounts();
  });
  const me = $derived(session.me);
  const look = $derived((me && looks[me.id]) || NO_LOOK);

  // Reports come one row per report: the next emotes with open reports, how many each.
  const reported = $derived.by(() => {
    if (!nextReports) return null;
    const by = new Map<string, { emote: NonNullable<(typeof nextReports)[number]['emote']>; n: number; reason: string }>();
    for (const r of nextReports) {
      if (!r.emote) continue;
      const g = by.get(r.emote.id);
      if (g) g.n++;
      else by.set(r.emote.id, { emote: r.emote, n: 1, reason: r.reason });
    }
    return [...by.values()].slice(0, NEXT);
  });

  // The shop today (local day): orders started, paid, revenue by currency, refunds.
  const today = $derived.by(() => {
    if (!orders) return null;
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const t0 = start.getTime();
    const mine = orders.filter((o) => Date.parse(o.createdAt) >= t0);
    const paid = orders.filter((o) => o.paidAt && Date.parse(o.paidAt) >= t0);
    const refunded = orders.filter((o) => o.refundedAt && Date.parse(o.refundedAt) >= t0);
    const sums = new Map<string, number>();
    for (const o of paid) {
      const cur = o.paidCurrency ?? o.currency;
      sums.set(cur, (sums.get(cur) ?? 0) + (o.paidTotalCents ?? o.amountCents));
    }
    return { started: mine.length, paid: paid.length, refunded: refunded.length, revenue: [...sums].map(([c, cents]) => price(cents, c)) };
  });

  const groups = $derived.by(() => {
    void session.me;
    return visibleSections(can);
  });
  function figure(s: Section): number | undefined {
    switch (s.href) {
      case '/admin/review': return reviewWaiting(counts, can);
      case '/admin/reports': return counts?.reportQueue?.total;
      case '/admin/shop/items': return shopCounts.items;
      case '/admin/shop/orders': return shopCounts.orders;
      case '/admin/shop/codes': return shopCounts.codes;
      case '/admin/cosmetics/paints': return catalog?.paints;
      case '/admin/cosmetics/badges': return catalog?.badges;
    }
    return undefined;
  }
  const loaded = $derived(counts !== null);

  type Quick = { href: string; label: string; icon: import('$lib/components/Icon.svelte').IconName };
  const quick = $derived.by<Quick[]>(() => {
    const q: Quick[] = [];
    if (review) q.push({ href: '/admin/review', label: 'Review the oldest upload', icon: 'inbox' });
    if (reports) q.push({ href: '/admin/reports', label: 'Work the reports', icon: 'flag' });
    if (can('site.announce')) q.push({ href: '/admin/announcements', label: 'Post an announcement', icon: 'message-square' });
    if (can('cosmetics.grant') || can('roles.assign')) q.push({ href: '/admin/grants', label: 'Give to many', icon: 'user-plus' });
    if (can('codes.manage')) q.push({ href: '/admin/shop/codes', label: 'Make redeem codes', icon: 'key' });
    if (cosmetics) q.push({ href: '/admin/cosmetics/paints/studio', label: 'New paint in the studio', icon: 'brush' });
    if (globalSet && counts) q.push({ href: `/me/sets/${counts.globalSet.id}`, label: 'Edit the global set', icon: 'layers' });
    return q;
  });
  const thumb = (e: Parameters<typeof size>[0]) => {
    const d = size(e);
    const w = Math.round((44 * d.width) / d.height);
    return { d, w, pic: pictureFor(e, display.dpr, false, w) };
  };
</script>

<svelte:head><title>Admin - cyex.tv</title></svelte:head>

<Split label="Today" open={true} stack="bottom" onclose={() => {}}>
  <header class="head">
    <h1>Overview</h1>
    <p class="hi">Signed in as {#if me}<PaintedName name={me.identities[0]?.displayName ?? ''} paint={look.paint} />{/if}</p>
  </header>

  <section class="adm-flat waits" aria-label="Waiting">
    {#if review}
      <div class="wait">
        <header>
          <a class="fig" href="/admin/review"><span class="adm-label"><Icon name="inbox" size={14} />Review</span>
            <span class="num" class:some={(reviewWaiting(counts, can) ?? 0) > 0}>{#if loaded}{count(reviewWaiting(counts, can) ?? 0)}{:else}<span class="ph ui-skeleton"></span>{/if}</span>
            <span class="sub">waiting for a verdict</span></a>
          <a class="ui-btn secondary small" href="/admin/review">Open queue<Icon name="arrow-right" size={14} /></a>
        </header>
        {#if nextReview === null}
          <ul class="adm-sheet tiles" aria-hidden="true">{#each { length: NEXT } as _, i (i)}<li class="adm-cell tile"><span class="ph-tile ui-skeleton"></span></li>{/each}</ul>
        {:else if nextReview.length}
          <ul class="adm-sheet tiles" aria-label="Next in review">
            {#each nextReview as e (e.id)}
              <li>
                <a class="adm-cell tile" href="/admin/review?q={encodeURIComponent(e.name)}">
                  <span class="pic">{#if ready(e)}{@const t = thumb(e)}<img src={t.pic.src} srcset={t.pic.srcset} sizes={t.pic.srcset ? `${t.w}px` : undefined} width={t.d.width} height={t.d.height} alt="" loading="lazy" />{/if}</span>
                  <span class="nm"><bdi>{clean(e.name)}</bdi></span>
                  <span class="sub"><bdi>{clean(e.owner?.displayName) || 'Unknown'}</bdi></span>
                  <span class="adm-flags"><time class="adm-num" datetime={e.createdAt}>{date(e.createdAt)}</time></span>
                </a>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="none">Nothing waiting. New uploads show up here.</p>
        {/if}
      </div>
    {/if}
    {#if reports}
      <div class="wait">
        <header>
          <a class="fig" href="/admin/reports"><span class="adm-label"><Icon name="flag" size={14} />Reports</span>
            <span class="num" class:some={(counts?.reportQueue?.total ?? 0) > 0}>{#if loaded}{count(counts?.reportQueue?.total ?? 0)}{:else}<span class="ph ui-skeleton"></span>{/if}</span>
            <span class="sub">open, oldest first</span></a>
          <a class="ui-btn secondary small" href="/admin/reports">Open reports<Icon name="arrow-right" size={14} /></a>
        </header>
        {#if reported === null}
          <ul class="adm-sheet tiles" aria-hidden="true">{#each { length: 3 } as _, i (i)}<li class="adm-cell tile"><span class="ph-tile ui-skeleton"></span></li>{/each}</ul>
        {:else if reported.length}
          <ul class="adm-sheet tiles" aria-label="Next reports">
            {#each reported as g (g.emote.id)}
              <li>
                <a class="adm-cell tile" href="/admin/reports?emote={encodeURIComponent(g.emote.id)}">
                  <span class="pic">{#if ready(g.emote)}{@const t = thumb(g.emote)}<img src={t.pic.src} srcset={t.pic.srcset} sizes={t.pic.srcset ? `${t.w}px` : undefined} width={t.d.width} height={t.d.height} alt="" loading="lazy" />{/if}</span>
                  <span class="nm"><bdi>{clean(g.emote.name)}</bdi></span>
                  <span class="sub"><bdi>{clean(g.reason)}</bdi></span>
                  <span class="adm-flags"><span class="warn"><span class="adm-num">{count(g.n)}</span> {g.n === 1 ? 'report' : 'reports'}</span></span>
                </a>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="none">No open reports.</p>
        {/if}
      </div>
    {/if}
  </section>

  {#if auditOk}
    <section class="adm-flat" aria-labelledby="ov-audit">
      <header><h2 class="adm-label" id="ov-audit">Recent staff activity</h2><a class="more" href="/admin/audit">Audit log<Icon name="chevron-right" size={14} /></a></header>
      {#if recent === null}
        <p class="none" aria-busy="true">Loading…</p>
      {:else if !recent.length}
        <p class="none">Nothing yet.</p>
      {:else}
        <ol class="log">
          {#each recent as r (r.id)}
            <li>
              <time class="adm-num" datetime={r.createdAt} title={dateTime(r.createdAt)}>{date(r.createdAt) === date(new Date().toISOString()) ? clock(r.createdAt) : date(r.createdAt)}</time>
              <span class="who"><AccountLink id={r.actorId} /></span>
              <span class="what">{actionText(r.action)}</span>
              <span class="tgt adm-num">{r.targetKind.toLowerCase()} {r.targetId}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  {/if}

  <section class="adm-flat" aria-labelledby="ov-all">
    <header><h2 class="adm-label" id="ov-all">Every section</h2></header>
    <ul class="adm-sheet index">
      {#each groups.flatMap((g) => g.items) as s (s.href)}
        {@const n = figure(s)}
        <li>
          <a class="adm-cell sec" href={s.href}>
            <span class="nm"><Icon name={s.icon} size={15} /><span>{s.label}</span>{#if n !== undefined}<span class="adm-num n">{count(n)}</span>{/if}</span>
            <span class="sub">{s.text}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>

  {#snippet panel()}
    <div class="adm-ip-head"><span class="adm-label">Today</span><span class="adm-num day">{date(new Date().toISOString())}</span></div>
    <div class="adm-ip-body">
      {#if ordersOk}
        <section class="adm-sec">
          <h3 class="adm-label">Shop today</h3>
          <dl class="adm-kv">
            <dt>Orders started</dt><dd>{today ? count(today.started) : '…'}</dd>
            <dt>Paid</dt><dd>{today ? count(today.paid) : '…'}</dd>
            <dt>Revenue, tax included</dt><dd>{today ? (today.revenue.join(' + ') || '0') : '…'}</dd>
            <dt>Refunded</dt><dd>{today ? count(today.refunded) : '…'}</dd>
            {#if shopCounts.items !== undefined}<dt>Items on sale</dt><dd>{count(shopCounts.items)}</dd>{/if}
          </dl>
          <a class="more" href="/admin/shop/orders">All orders<Icon name="chevron-right" size={14} /></a>
        </section>
      {/if}
      {#if quick.length}
        <section class="adm-sec">
          <h3 class="adm-label">Quick actions</h3>
          <ul class="quick">
            {#each quick as q (q.href)}
              <li><a href={q.href}><Icon name={q.icon} size={15} /><span>{q.label}</span><Icon name="chevron-right" size={14} /></a></li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  {/snippet}
</Split>

<style>
  h1, h2, h3, p { margin: 0; }
  .head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 12px; min-height: 40px; margin-bottom: 8px; }
  h1 { font-size: var(--t-page); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; }
  .hi { font-size: var(--t-small); color: var(--muted); }
  .hi :global(.pn) { font-weight: 700; }

  .waits { gap: 20px; }
  .wait { display: grid; gap: 10px; min-width: 0; }
  .wait > header { display: flex; align-items: flex-end; justify-content: space-between; gap: 12px; }
  .fig { display: grid; grid-template-columns: auto auto; align-items: baseline; column-gap: 12px; color: var(--text); text-decoration: none; }
  .fig .adm-label { grid-column: 1 / -1; display: flex; align-items: center; gap: 6px; }
  .fig .adm-label :global(svg) { color: var(--accent-text); }
  .num { display: block; height: 36px; font-family: var(--font-mono); font-size: 30px; font-weight: 600; line-height: 36px; font-variant-numeric: tabular-nums; color: var(--muted); }
  .num.some { color: var(--text); }
  .ph { display: block; width: 56px; height: 28px; margin-top: 4px; border-radius: var(--radius); }
  .sub { font-size: var(--t-fine); color: var(--dim); }
  .fig:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }

  .tiles { --cell: 136px; }
  .tile { height: 128px; }
  .tile .pic { height: 52px; margin-bottom: 4px; }
  .tile .pic img { max-height: 44px; }
  .ph-tile { display: block; height: 100%; border-radius: var(--radius); }
  .none { font-size: var(--t-small); color: var(--muted); }

  .adm-flat > header { justify-content: space-between; }
  .more { display: inline-flex; align-items: center; gap: 4px; font-size: var(--t-small); font-weight: 600; color: var(--muted); text-decoration: none; }
  .log { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  .log li { display: grid; grid-template-columns: 84px minmax(90px, 160px) minmax(0, 1fr) minmax(0, 220px); align-items: center; gap: 12px; min-height: 40px;
    border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .log time { font-size: var(--t-fine); color: var(--dim); }
  .who, .what, .tgt { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who { font-weight: 600; }
  .tgt { font-size: 12px; color: var(--dim); text-align: right; }
  @media (max-width: 719.98px) {
    .log li { grid-template-columns: 64px minmax(0, 1fr); padding: 6px 0; }
    .what { grid-column: 2; }
    .tgt { display: none; }
  }

  .index { --cell: 240px; }
  .sec { min-height: 64px; }
  .sec .nm :global(svg) { flex: none; color: var(--dim); }
  .sec .n { margin-left: auto; font-weight: 500; color: var(--dim); }

  .day { margin-left: auto; font-size: var(--t-fine); color: var(--muted); }
  .adm-ip-head { gap: 8px; padding-right: 16px; }
  .quick { list-style: none; margin: 0; padding: 0; }
  .quick li + li { border-top: 1px solid var(--border); }
  .quick a { display: flex; align-items: center; gap: 10px; min-height: 44px; color: var(--text); font-size: var(--t-small); font-weight: 600; text-decoration: none; }
  .quick a span { flex: 1; min-width: 0; }
  .quick a :global(svg) { flex: none; color: var(--dim); }
  .quick a :global(svg:first-child) { color: var(--accent-text); }
  .quick a:focus-visible, .more:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) {
    .quick a:hover, .more:hover { color: var(--text); text-decoration: underline; text-underline-offset: 3px; }
    .fig:hover .num.some { color: var(--accent-text); }
  }
</style>
