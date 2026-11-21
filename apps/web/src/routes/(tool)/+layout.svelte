<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and frames the page: Settings and, under /admin, the admin pages sit in a RailLayout with their
  // rail; every other /me page draws its own Inspector frame (MeFrame, InspectorLayout), the same
  // shape as your profile.
  import { onMount } from 'svelte';
  import { quiet } from '$lib/quiet';
  import { useToolButtons } from '$lib/components/btn';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { settingsRail, isSettingsPath } from '$lib/account/settings';
  import Icon from '$lib/components/Icon.svelte';
  import type { IconName } from '$lib/components/Icon.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { markCurrent, type RailGroup } from '$lib/shell/rail';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { can, isAdmin, loadMe, session } from '$lib/tool/session.svelte';
  import { USER_PAGE_PERMISSIONS } from '$lib/admin/permissions';

  let { children } = $props();
  useToolButtons(); // the tool pages' buttons are .ui-btn; shared components follow
  let problem = $state(false);

  const path = $derived(page.url.pathname);
  // Under /admin with staff permissions: the admin rail.
  const adminPath = $derived(path === '/admin' || path.startsWith('/admin/'));
  const admin = $derived(adminPath && isAdmin(session.me));

  const row = (href: string, label: string, icon: IconName, show = true, count?: number) => (show ? [{ href, label, icon, count }] : []);

  // Settings (and the Editors page it lists) have their own rail; the other /me pages frame themselves.
  const settings = $derived(isSettingsPath(path));
  // No rail: a /me page outside Settings (staff under /admin get the admin rail; before the
  // account is known, the path says which).
  const framed = $derived(!settings && (session.me ? !admin : !adminPath));

  // Open tickets for the admin rail's counts.
  let counts = $state<AdminCountsQuery | null>(null);
  $effect(() => {
    if (!admin) return;
    const review = can('emotes.review');
    const reports = review || can('emotes.takedown') || can('reports.manage');
    if (!review && !reports) return;
    void quiet(gql(ADMIN_COUNTS, { review, reports }).then((r) => (counts = r)), 'admin counts');
  });

  $effect(() => {
    if (admin) loadShopCounts();
  });

  const adminRail = $derived.by<RailGroup[]>(() => {
    void session.me; // permissions come from the account
    const tickets = can('emotes.review') || can('emotes.takedown') || can('reports.manage');
    const cosmetics = can('cosmetics.manage') || can('cosmetics.grant');
    const groups: RailGroup[] = [
      { items: row('/admin', 'Overview', 'gauge') },
      {
        toggle: { label: 'Tickets', icon: 'ticket' },
        items: [
          ...row('/admin/review', 'Review', 'inbox', can('emotes.review'), counts?.reviewQueue?.total),
          ...row('/admin/reports', 'Reports', 'flag', tickets, counts?.reportQueue?.total),
        ],
      },
      {
        toggle: { label: 'Cosmetics', icon: 'palette' },
        items: [
          ...row('/admin/cosmetics/paints', 'Paints', 'brush', cosmetics),
          ...row('/admin/cosmetics/badges', 'Badges', 'badge', cosmetics),
          ...row('/admin/cosmetics/owners', 'Owners', 'users', can('cosmetics.grant')),
        ],
      },
      {
        toggle: { label: 'Shop', icon: 'shopping-bag' },
        items: [
          ...row('/admin/shop/items', 'Items', 'tag', can('shop.manage'), shopCounts.items),
          ...row('/admin/shop/orders', 'Orders', 'list', can('shop.orders'), shopCounts.orders),
          ...row('/admin/shop/codes', 'Codes', 'key', can('codes.manage'), shopCounts.codes),
        ],
      },
      {
        toggle: { label: 'People', icon: 'users' },
        items: [
          ...row('/admin/users', 'Users', 'users', USER_PAGE_PERMISSIONS.some(can)),
          ...row('/admin/roles', 'Roles', 'key', can('roles.manage') || can('roles.assign')),
          ...row('/admin/grants', 'Give to many', 'user-plus', can('cosmetics.grant') || can('roles.assign') || can('roles.manage')),
          ...row('/admin/entitlements', 'Entitlement graph', 'network', can('entitlements.view') || can('cosmetics.grant')),
        ],
      },
      {
        toggle: { label: 'System', icon: 'settings' },
        items: [...row('/admin/announcements', 'Announcements', 'message-square', can('site.announce')), ...row('/admin/audit', 'Audit log', 'scroll', can('audit.view'))],
      },
    ];
    return markCurrent(groups.filter((g) => g.items.length), path, ['/admin']);
  });

  // A retry keeps the problem on screen until the answer is in, so the page's Try again button can tell it worked.
  async function signIn(fresh = false) {
    try {
      await loadMe({ fresh });
      problem = false;
    } catch (e) {
      if (e instanceof ApiError && e.code === 'signed_out') {
        await goto(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`, { replaceState: true });
        return;
      }
      problem = true;
    }
  }
  onMount(() => signIn());
</script>

{#snippet body()}
  {#if problem}
    <PageTitle title={problemTitle('server')} />
    <Problem cause="server" level={1} onretry={() => signIn(true)} />
  {:else if !session.me}
    <PageTitle title="Loading - cyex.tv" />
    <State kind="loading" title="Loading your account…" />
  {:else}
    {@render children()}
  {/if}
{/snippet}

{#if framed}
  <!-- A /me page draws its own frame (with the page wrapper); the sign-in states get a plain one. -->
  <div class="ui-page" class:pending={!problem && !session.me}>
    {#if problem || !session.me}<div class="wrap me-wait">{@render body()}</div>{:else}{@render children()}{/if}
  </div>
{:else}
<RailLayout>
  {#snippet rail()}
    {#if admin}
      <SideRail label="Admin" groups={adminRail} fill>
        {#snippet head()}
          <div class="title"><strong>Admin</strong><span class="ui-chip">Staff</span></div>
        {/snippet}
        {#snippet foot()}
          {#if session.me}<a class="rail-foot" href="/users/{session.me.id}"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/if}
        {/snippet}
      </SideRail>
    {:else if session.me && settings}
      <SideRail label="Settings" groups={settingsRail(path)} fill>
        {#snippet head()}<div class="title"><strong>Settings</strong></div>{/snippet}
        {#snippet foot()}<a class="rail-foot" href="/users/{session.me?.id}?s=overview"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/snippet}
      </SideRail>
    {:else}
      <SideRail label="Your account" wait fill>
        {#snippet head()}
          <div class="me">
            {#if problem}
              <span class="ph-av" aria-hidden="true"></span>
              <strong class="plain">Your account</strong>
            {:else}
              <span class="ph-av ui-skeleton" aria-hidden="true"></span>
              <span class="ph-line ui-skeleton" aria-hidden="true"></span>
            {/if}
          </div>
        {/snippet}
      </SideRail>
    {/if}
  {/snippet}
  <div class="ui-page" class:pending={!problem && !session.me}>{@render body()}</div>
</RailLayout>
{/if}

<style>
  /* While the account is unknown the page is a loading line: the footer waits (hidden, not
     removed) so the page growing doesn't shove a visible footer down. */
  :global(main:has(.ui-page.pending) + .footer) { visibility: hidden; }
  .me-wait { padding-top: var(--rail-gap); padding-bottom: 48px; }
  .me { display: flex; align-items: center; gap: 12px; min-height: 40px; }
  .plain { font-size: var(--t-ui); font-weight: 600; color: var(--muted); }
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 28px; padding-left: 10px; }
  /* The rail's name is an uppercase label, as the browse rail's SETS. */
  .title strong { font-size: var(--text-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .ph-av { flex: none; width: 40px; height: 40px; border-radius: 50%; background: var(--surface-hover); }
  .ph-line { display: block; width: 110px; height: 12px; border-radius: var(--radius); }
</style>
