<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and puts the page in a RailLayout with the /me rail or, under /admin, the admin frame (AdminShell).
  import { onMount } from 'svelte';
  import { quiet } from '$lib/quiet';
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
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import type { RailGroup } from '$lib/shell/rail';
  import AdminShell from '$lib/admin/shell/AdminShell.svelte';
  import { adminRail as adminRailFor } from '$lib/admin/shell/sections';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { can, isAdmin, loadMe, session } from '$lib/tool/session.svelte';

  let { children } = $props();
  useToolButtons(); // the tool pages' buttons are .ui-btn; shared components follow
  let problem = $state(false);

  const path = $derived(page.url.pathname);
  const who = $derived(session.me?.identities[0]);
  // Under /admin with staff permissions: the admin rail. Anything else: the /me rail.
  const adminPath = $derived(path === '/admin' || path.startsWith('/admin/'));
  const admin = $derived(adminPath && isAdmin(session.me));

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
    void quiet(profileOf(id).then((p) => (meProfile = p)), 'your profile');
  });

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

  // The admin rail: the sections your permissions open (lib/admin/shell/sections), with the open tickets and shop counts.
  const adminRail = $derived.by<RailGroup[]>(() => {
    void session.me; // permissions come from the account
    const c = { review: counts?.reviewQueue?.total, reports: counts?.reportQueue?.total, items: shopCounts.items, orders: shopCounts.orders, codes: shopCounts.codes };
    return adminRailFor(can, c, path);
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
    <ProfileHeader account={meAccount} relation={meRelation} profile={meProfile} heading={false}>
      {#snippet actions()}<a class="btn btn-s" href="/me/settings"><Icon name="settings" size={15} />Settings</a>{/snippet}
    </ProfileHeader>
  {:else}
    <ProfileHeader account={{ id: '', displayName: '' }} relation={meRelation} profile={null} loading />
  {/if}
{/snippet}

{#snippet pageBody()}
  <div class="ui-page" class:pending={!problem && !session.me}>
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
{/snippet}

{#if adminPath && (admin || (!session.me && !problem))}
  <!-- Staff under /admin: the admin frame (full-height rail, no band). Before the account is known
       the path decides, so the frame is there from the first paint and its rows fill in. -->
  <AdminShell groups={adminRail} me={session.me?.id}>{@render pageBody()}</AdminShell>
{:else}
<!-- The band is there from the first paint (blank until your account is known), except where the
     page has no band at all: settings and the staff pages. Before the account is known, the path
     says which; once it is, whether you are staff does. -->
<RailLayout top={!settings && !problem && (session.me ? !admin : !adminPath) ? band : undefined}>
  {#snippet rail()}
    {#if meAccount && settings}
      <SideRail label="Settings" groups={settingsRail(path)} fill>
        {#snippet head()}<div class="title"><strong>Settings</strong></div>{/snippet}
        {#snippet foot()}<a class="rail-foot" href="/users/{meAccount.id}?s=overview"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/snippet}
      </SideRail>
    {:else if meAccount}
      <!-- The same band and rail as your public profile (/users/<id>), so moving between them never shifts them. -->
      <ProfileRail account={meAccount} current={railKeyForPath(path)} activity={ACTIVITY_READY} />
    {:else}
      <SideRail label="Your account" wait>
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
  {@render pageBody()}
</RailLayout>
{/if}

<style>
  /* While the account is unknown the page is a loading line: the footer waits (hidden, not
     removed) so the page growing doesn't shove a visible footer down. */
  :global(main:has(.ui-page.pending) + .footer) { visibility: hidden; }
  .me { display: flex; align-items: center; gap: 12px; min-height: 40px; }
  .plain { font-size: var(--t-ui); font-weight: 600; color: var(--muted); }
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding-left: 8px; }
  .title strong { font-size: var(--t-ui); font-weight: 600; }
  .ph-av { flex: none; width: 40px; height: 40px; border-radius: 50%; background: var(--surface-hover); }
  .ph-line { display: block; width: 110px; height: 12px; border-radius: var(--radius); }
</style>
