<script lang="ts">
  // The signed-in pages (your emotes, sets, channels, account, admin). The root layout already
  // draws the one AppHeader and Footer; this layout only signs you in (or sends you to /signin)
  // and frames the page: under /admin the admin layout's frame (AdminShell); Settings in a RailLayout with
  // its rail; every other /me page draws its own Inspector frame (MeFrame, InspectorLayout), the
  // same shape as your profile.
  import { onMount } from 'svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError } from '$lib/api';
  import { settingsRail, isSettingsPath } from '$lib/account/settings';
  import Icon from '$lib/components/Icon.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import WorkspaceLayout from '$lib/shell/WorkspaceLayout.svelte';
  import SettingsPanel from '$lib/account/SettingsPanel.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { isAdmin, loadMe, session } from '$lib/tool/session.svelte';

  let { children } = $props();
  useToolButtons(); // the tool pages' buttons are .ui-btn; shared components follow
  let problem = $state(false);

  const path = $derived(page.url.pathname);
  // Under /admin with staff permissions: the admin layout (admin/+layout.svelte) draws the frame.
  const adminPath = $derived(path === '/admin' || path.startsWith('/admin/'));
  const admin = $derived(adminPath && isAdmin(session.me));

  // Settings (and the Editors page it lists) have their own rail; the other /me pages frame themselves.
  const settings = $derived(isSettingsPath(path));
  // No rail: a /me page outside Settings (staff under /admin get the admin rail; before the
  // account is known, the path says which).
  const framed = $derived(!settings && (session.me ? !admin : !adminPath));

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

{#if adminPath && (admin || (!session.me && !problem))}
  <!-- Staff under /admin: the admin layout draws its own frame (full-height rail, no band), and its
       loading line before the account is known, so the frame is there from the first paint. -->
  {@render children()}
{:else if framed}
  <!-- A /me page draws its own frame (with the page wrapper); the sign-in states get a plain one. -->
  <div class="ui-page" class:pending={!problem && !session.me}>
    {#if problem || !session.me}<div class="wrap me-wait">{@render body()}</div>{:else}{@render children()}{/if}
  </div>
{:else if settings}
  <!-- Settings: the Workspace (redesign round 2): the flush rail, the section, and on wide
       screens a panel with what the section does and your account's facts. -->
  {#snippet settingsRail_()}
    <SideRail label="Settings" groups={settingsRail(path)} fill>
      {#snippet head()}<div class="title"><strong>Settings</strong></div>{/snippet}
      {#snippet foot()}<a class="rail-foot" href="/users/{session.me?.id ?? ''}?s=overview"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>{/snippet}
    </SideRail>
  {/snippet}
  {#snippet settingsPanel()}<SettingsPanel />{/snippet}
  <WorkspaceLayout rail={settingsRail_} panel={session.me ? settingsPanel : undefined} title="About" label="About this section" narrow="after" fold testid="settings-panel">
    <div class="ui-page" class:pending={!problem && !session.me}>{@render body()}</div>
  </WorkspaceLayout>
{:else}
<RailLayout>
  {#snippet rail()}
    {#if session.me && settings}
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
