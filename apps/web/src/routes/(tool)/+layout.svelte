<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and puts the page in a RailLayout with the /me rail or, under /admin, the admin rail.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { relationTo } from '$lib/profile/data.svelte';
  import ProfileRail from '$lib/profile/ProfileRail.svelte';
  import { railKeyForPath } from '$lib/profile/rows';
  import Icon from '$lib/components/Icon.svelte';
  import type { IconName } from '$lib/components/Icon.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { markCurrent, type RailGroup } from '$lib/shell/rail';
  import State from '$lib/components/State.svelte';
  import { can, isAdmin, loadMe, session } from '$lib/tool/session.svelte';

  let { children } = $props();
  let problem = $state<string | null>(null);

  const path = $derived(page.url.pathname);
  const who = $derived(session.me?.identities[0]);
  // Under /admin with staff permissions: the admin rail. Anything else: the /me rail.
  const admin = $derived((path === '/admin' || path.startsWith('/admin/')) && isAdmin(session.me));

  const row = (href: string, label: string, icon: IconName, show = true, count?: number) => (show ? [{ href, label, icon, count }] : []);

  // The /me pages sit in your profile's rail (your profile is your dashboard), the row they belong to marked.
  const meAccount = $derived(session.me ? { id: session.me.id, displayName: who?.displayName ?? '', avatar: who?.avatar } : null);
  const meRelation = $derived(relationTo(session.me?.id ?? '', session.me, null));

  // Open tickets for the admin rail's counts.
  let counts = $state<AdminCountsQuery | null>(null);
  $effect(() => {
    if (!admin) return;
    const review = can('emotes.review');
    const reports = review || can('emotes.takedown');
    if (!review && !reports) return;
    gql(ADMIN_COUNTS, { review, reports })
      .then((r) => (counts = r))
      .catch(() => {});
  });

  const adminRail = $derived.by<RailGroup[]>(() => {
    void session.me; // permissions come from the account
    const tickets = can('emotes.review') || can('emotes.takedown');
    const cosmetics = can('cosmetics.manage') || can('cosmetics.grant');
    // Reserved for later (another branch): a "Shop" group with Items, Orders and Codes.
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
        caption: 'People',
        items: [
          ...row('/admin/users', 'Users', 'users', can('users.view') || can('users.restrict') || can('roles.manage') || can('cosmetics.grant')),
          ...row('/admin/roles', 'Roles', 'key', can('roles.manage')),
        ],
      },
      { caption: 'System', items: row('/admin/audit', 'Audit log', 'scroll', can('audit.view')) },
    ];
    return markCurrent(groups.filter((g) => g.items.length), path, ['/admin']);
  });

  let trying = $state(false);
  async function signIn(fresh = false) {
    trying = true;
    problem = null;
    try {
      await loadMe({ fresh });
    } catch (e) {
      if (e instanceof ApiError && e.code === 'signed_out') {
        await goto(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`, { replaceState: true });
        return;
      }
      problem = 'cyex.tv didn’t answer, so your account couldn’t load. It’s usually back within a minute.';
    } finally {
      trying = false;
    }
  }
  onMount(() => signIn());
</script>

<RailLayout>
  {#snippet rail()}
    {#if admin}
      <SideRail label="Admin" groups={adminRail}>
        {#snippet head()}
          <div class="title"><strong>Admin</strong><span class="ui-chip">Staff</span></div>
        {/snippet}
        {#snippet foot()}
          {#if session.me}<a class="rail-foot" href="/users/{session.me.id}"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/if}
        {/snippet}
      </SideRail>
    {:else if meAccount}
      <ProfileRail account={meAccount} relation={meRelation} current={railKeyForPath(path)} />
    {:else}
      <SideRail label="Your account">
        {#snippet head()}
          <div class="me">
            {#if problem}
              <span class="ph-av" aria-hidden="true"></span>
              <span class="id">
                <strong>Not loaded</strong>
                <button class="retry" type="button" onclick={() => signIn(true)} disabled={trying}>{trying ? 'Trying…' : 'Try again'}</button>
              </span>
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
      <State kind="error" level={1} title="This page can’t load right now.">
        {problem}
        {#snippet actions()}
          <button class="ui-btn primary" type="button" onclick={() => signIn(true)} disabled={trying}>{trying ? 'Trying…' : 'Try again'}</button>
        {/snippet}
      </State>
    {:else if !session.me}
      <State kind="loading" title="Loading your account…" />
    {:else}
      {@render children()}
    {/if}
  </div>
</RailLayout>

<style>
  .me { display: flex; align-items: center; gap: 12px; min-height: 40px; }
  .id { display: grid; min-width: 0; line-height: 1.3; }
  .id strong { font-size: var(--t-ui); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding-left: 8px; }
  .title strong { font-size: var(--t-ui); font-weight: 600; }
  .ph-av { flex: none; width: 40px; height: 40px; border-radius: 50%; background: var(--surface-hover); }
  .retry { justify-self: start; display: inline-flex; align-items: center; min-height: 40px; margin-block: -10px; padding: 0;
    border: 0; background: none; font: inherit; font-size: var(--t-fine); font-weight: 500; color: var(--accent-text); cursor: pointer; }
  .retry:disabled { color: var(--muted); cursor: default; }
  .ph-line { display: block; width: 110px; height: 12px; border-radius: var(--radius); }
</style>
