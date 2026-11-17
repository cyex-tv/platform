<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and puts the page in a RailLayout with the /me rail or, under /admin, the admin rail.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import type { IconName } from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import { ACCOUNT, LIBRARY, platformName } from '$lib/shell/account';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { markCurrent, type RailGroup } from '$lib/shell/rail';
  import { can, isAdmin, loadMe, session } from '$lib/tool/session.svelte';

  let { children } = $props();
  let problem = $state<string | null>(null);

  const path = $derived(page.url.pathname);
  const who = $derived(session.me?.identities[0]);
  const name = $derived(clean(who?.displayName));
  // Under /admin with staff permissions: the admin rail. Anything else: the /me rail.
  const admin = $derived((path === '/admin' || path.startsWith('/admin/')) && isAdmin(session.me));

  const row = (href: string, label: string, icon: IconName, show = true) => (show ? [{ href, label, icon }] : []);

  const meRail = $derived<RailGroup[]>(
    markCurrent(
      [
        { caption: 'Library', items: LIBRARY.map(({ href, label, icon }) => ({ href, label, icon })) },
        { caption: 'Settings', items: row(ACCOUNT.href, ACCOUNT.label, ACCOUNT.icon) },
      ],
      path,
    ),
  );

  const adminRail = $derived.by<RailGroup[]>(() => {
    void session.me; // permissions come from the account
    const groups: RailGroup[] = [
      {
        caption: 'Moderation',
        items: [
          ...row('/admin', 'Overview', 'gauge'),
          ...row('/admin/review', 'Review', 'inbox', can('emotes.review')),
          ...row('/admin/reports', 'Reports', 'flag', can('emotes.review') || can('emotes.takedown')),
        ],
      },
      {
        caption: 'People',
        items: [
          ...row('/admin/users', 'Users', 'users', can('users.view') || can('users.restrict') || can('roles.manage')),
          ...row('/admin/roles', 'Roles', 'key', can('roles.manage')),
        ],
      },
      { caption: 'System', items: row('/admin/audit', 'Audit log', 'scroll', can('audit.view')) },
    ];
    return markCurrent(groups.filter((g) => g.items.length), path, ['/admin']);
  });

  onMount(async () => {
    try {
      await loadMe();
    } catch (e) {
      if (e instanceof ApiError && e.code === 'signed_out') {
        await goto(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`, { replaceState: true });
        return;
      }
      problem = e instanceof Error ? e.message : 'cyex.tv can’t be reached right now.';
    }
  });
</script>

<RailLayout>
  {#snippet rail()}
    {#if admin}
      <SideRail label="Admin" groups={adminRail}>
        {#snippet head()}
          <div class="title"><strong>Admin</strong><span class="ui-chip">Staff</span></div>
        {/snippet}
      </SideRail>
    {:else}
      <SideRail label="Your account" groups={meRail}>
        {#snippet head()}
          <div class="me">
            {#if session.me}
              <Avatar src={who?.avatar} {name} size={40} />
              <span class="id">
                <strong><bdi>{name}</bdi></strong>
                <span>{platformName(who?.platform) ?? ''}</span>
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
      <div class="ui-empty" role="alert">
        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
        <p>{problem}</p>
        <button class="ui-btn secondary" type="button" onclick={() => location.reload()}>Try again</button>
      </div>
    {:else if !session.me}
      <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span><p>Loading…</p></div>
    {:else}
      {@render children()}
    {/if}
  </div>
</RailLayout>

<style>
  .me { display: flex; align-items: center; gap: 12px; min-height: 40px; }
  .id { display: grid; min-width: 0; line-height: 1.3; }
  .id strong { font-size: var(--t-ui); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .id span { font-size: var(--t-fine); color: var(--muted); }
  .title { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding-left: 8px; }
  .title strong { font-size: var(--t-ui); font-weight: 600; }
  .ph-av { flex: none; width: 40px; height: 40px; border-radius: 50%; }
  .ph-line { display: block; width: 110px; height: 12px; border-radius: var(--radius); }
</style>
