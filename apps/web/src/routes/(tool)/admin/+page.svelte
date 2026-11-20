<script lang="ts">
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { can, session } from '$lib/tool/session.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { fail } from '$lib/tool/notices.svelte';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { USER_PAGE_PERMISSIONS } from '$lib/admin/permissions';

  let counts = $state<AdminCountsQuery | null>(null);
  const review = can('emotes.review');
  const reports = review || can('emotes.takedown') || can('reports.manage');

  $effect(() => {
    gql(ADMIN_COUNTS, { review, reports })
      .then((r) => (counts = r))
      .catch(fail);
  });

  // The catalog's size for the cosmetics cards, and your own look for the welcome line.
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

  type Card = { href: string; title: string; text: string; icon: IconName; figure?: number; show: boolean };
  const cards = $derived(
    [
      { href: '/admin/review', title: 'Review queue', icon: 'inbox' as const, text: 'Pending uploads, oldest first.', figure: counts?.reviewQueue?.total, show: review },
      { href: '/admin/reports', title: 'Reports', icon: 'flag' as const, text: 'Open reports. Dismiss them or take the emote down.', figure: counts?.reportQueue?.total, show: reports },
      { href: '/admin/cosmetics/paints', title: 'Paints', icon: 'brush' as const, text: 'Name paints: who owns them, give and take away.', figure: catalog?.paints, show: cosmetics },
      { href: '/admin/cosmetics/badges', title: 'Badges', icon: 'badge' as const, text: 'Chat badges: make, edit, give and take away.', figure: catalog?.badges, show: cosmetics },
      { href: '/admin/shop/items', title: 'Shop items', icon: 'tag' as const, text: 'What is on sale: bundles of badges and paints, prices, sale windows, stock.', figure: shopCounts.items, show: can('shop.manage') },
      { href: '/admin/shop/orders', title: 'Orders', icon: 'list' as const, text: 'Every purchase and gift. Refund a paid order.', figure: shopCounts.orders, show: can('shop.orders') },
      { href: '/admin/shop/codes', title: 'Codes', icon: 'key' as const, text: 'Batches of redeem codes: make, hand out, turn off.', figure: shopCounts.codes, show: can('codes.manage') },
      { href: '/admin/users', title: 'Users', icon: 'users' as const, text: 'Find someone; their whole account on one page: restrictions, roles, billing, notes.', show: USER_PAGE_PERMISSIONS.some(can) },
      { href: '/admin/roles', title: 'Roles', icon: 'key' as const, text: 'Who may do what. You manage roles below your own.', show: can('roles.manage') || can('roles.assign') },
      { href: '/admin/entitlements', title: 'Entitlement graph', icon: 'network' as const, text: 'How each badge, paint and role was given, and by whom.', show: can('entitlements.view') || can('cosmetics.grant') },
      { href: '/admin/announcements', title: 'Announcements', icon: 'message-square' as const, text: 'One line under the header for everyone: news, maintenance or an event.', show: can('site.announce') },
      { href: '/admin/audit', title: 'Audit log', icon: 'scroll' as const, text: 'Every audited change, newest first.', show: can('audit.view') },
      { href: counts ? `/me/sets/${counts.globalSet.id}` : '/admin', title: 'Global set', icon: 'layers' as const, text: 'Shown in every channel that uses cyex.tv.', show: can('global_set.edit') },
    ].filter((c) => c.show),
  );
</script>

<svelte:head><title>Admin - cyex.tv</title></svelte:head>

<header class="welcome">
  <h1>Welcome back{#if me}, <PaintedName name={me.identities[0]?.displayName ?? ''} paint={look.paint} />{/if}</h1>
  <p>What your roles let you do, and what waits for you.</p>
</header>

<ul class="cards">
  {#each cards as c (c.href)}
    <li>
      <a class="card" href={c.href}>
        <span class="icon" aria-hidden="true"><Icon name={c.icon} size={18} /></span>
        <span class="text">
          <strong>{c.title}</strong>
          <span class="ui-hint">{c.text}</span>
        </span>
        {#if c.figure !== undefined}<span class="ui-mono figure" aria-hidden="true" class:some={c.figure > 0 && (c.href === '/admin/review' || c.href === '/admin/reports')}>{count(c.figure)}</span><span class="sr-only">, {count(c.figure)}</span>{/if}
      </a>
    </li>
  {/each}
</ul>

<style>
  .welcome { margin-bottom: 24px; }
  .welcome h1 { margin: 0; font-size: var(--t-page); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; overflow-wrap: anywhere; }
  .welcome p { margin: 8px 0 0; color: var(--muted); font-size: var(--t-ui); }
  .cards { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr)); gap: 12px; }
  .card { display: flex; align-items: flex-start; gap: 12px; height: 100%; padding: 14px; color: var(--text); text-decoration: none;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card);
    transition: background-color var(--fast), border-color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .card:hover { background: var(--surface-hover); border-color: var(--border-strong); } }
  .card:active { opacity: .6; transition: none; }
  .card:focus-visible { outline-offset: 2px; }
  .icon { display: grid; place-items: center; flex: none; width: 32px; height: 32px; border-radius: var(--radius);
    background: var(--elevated-2); color: var(--muted); }
  .text { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .text strong { font-weight: 600; }
  .figure { flex: none; align-self: center; min-width: 28px; padding: 2px 8px; border-radius: var(--radius); text-align: center;
    font-size: var(--t-small); font-weight: 600; background: var(--neutral-chip); color: var(--muted); }
  .figure.some { background: var(--accent-dim); color: var(--accent-text); }
</style>
