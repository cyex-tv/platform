<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and puts the page in a RailLayout with the /me rail or, under /admin, the admin rail.
  import { onMount } from 'svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { profileOf, relationTo, type PublicProfile } from '$lib/profile/data.svelte';
  import ProfileHeader from '$lib/profile/ProfileHeader.svelte';
  import ProfileRail from '$lib/profile/ProfileRail.svelte';
  import { ACTIVITY_READY } from '$lib/profile/activity';
  import { railKeyForPath } from '$lib/profile/rows';
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
  const who = $derived(session.me?.identities[0]);
  // Under /admin with staff permissions: the admin rail. Anything else: the /me rail.
  const admin = $derived((path === '/admin' || path.startsWith('/admin/')) && isAdmin(session.me));

  const row = (href: string, label: string, icon: IconName, show = true, count?: number) => (show ? [{ href, label, icon, count }] : []);

  // The /me pages sit in your profile's rail (your profile is your dashboard), the row they belong to marked.
  const meAccount = $derived(session.me ? { id: session.me.id, displayName: who?.displayName ?? '', avatar: who?.avatar } : null);
  const meRelation = $derived(relationTo(session.me?.id ?? '', session.me, null));
  // Settings (and the Editors page it lists) have their own rail; the other /me pages sit under
  // your profile's band and rail.
  const settings = $derived(isSettingsPath(path));
  let meProfile = $state<PublicProfile | null>(null);
  $effect(() => {
    const id = session.me?.id;
    if (!id) return;
    profileOf(id)
      .then((p) => (meProfile = p))
      .catch(() => {});
  });

  // Open tickets for the admin rail's counts.
  let counts = $state<AdminCountsQuery | null>(null);
  $effect(() => {
    if (!admin) return;
    const review = can('emotes.review');
    const reports = review || can('emotes.takedown') || can('reports.manage');
    if (!review && !reports) return;
    gql(ADMIN_COUNTS, { review, reports })
      .then((r) => (counts = r))
      .catch(() => {});
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
          ...row('/admin/entitlements', 'Entitlement graph', 'network', can('entitlements.view') || can('cosmetics.grant')),
        ],
      },
      { toggle: { label: 'System', icon: 'settings' }, items: row('/admin/audit', 'Audit log', 'scroll', can('audit.view')) },
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

{#snippet band()}
  {#if meAccount}
    <ProfileHeader account={meAccount} relation={meRelation} profile={meProfile}>
      {#snippet actions()}<a class="btn btn-s" href="/me/settings"><Icon name="settings" size={15} />Settings</a>{/snippet}
    </ProfileHeader>
  {/if}
{/snippet}

<RailLayout top={!admin && !settings && meAccount ? band : undefined}>
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
    {:else if meAccount && settings}
      <SideRail label="Settings" groups={settingsRail(path)} fill>
        {#snippet head()}<div class="title"><strong>Settings</strong></div>{/snippet}
        {#snippet foot()}<a class="rail-foot" href="/users/{meAccount.id}?s=overview"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/snippet}
      </SideRail>
    {:else if meAccount}
      <!-- The same band and rail as your public profile (/users/<id>), so moving between them never shifts them. -->
      <ProfileRail account={meAccount} relation={meRelation} current={railKeyForPath(path)} activity={ACTIVITY_READY} />
    {:else}
      <SideRail label="Your account">
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
  <div class="ui-page">
    {#if problem}
      <PageTitle title={problemTitle('server')} />
      <Problem cause="server" level={1} onretry={() => signIn(true)} />
    {:else if !session.me}
      <PageTitle title="Loading - cyex.tv" />
      <State kind="loading" title="Loading your account…" />
    {:else}
      {@render children()}
    {/if}
  </div>
</RailLayout>

<style>
  .me { display: flex; align-items: center; gap: 12px; min-height: 40px; }
  .plain { font-size: var(--t-ui); font-weight: 600; color: var(--muted); }
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding-left: 8px; }
  .title strong { font-size: var(--t-ui); font-weight: 600; }
  .ph-av { flex: none; width: 40px; height: 40px; border-radius: 50%; background: var(--surface-hover); }
  .ph-line { display: block; width: 110px; height: 12px; border-radius: var(--radius); }
</style>
