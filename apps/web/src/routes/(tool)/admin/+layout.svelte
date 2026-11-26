<script lang="ts">
  // The admin frame and gate. The signed-in layout (../+layout.svelte) signs you in and hands
  // staff (and anyone before the account is known) to this layout, which draws the admin rail (the
  // sections your permissions open, lib/admin/shell/sections, with the open tickets and shop
  // counts) and keeps AdminShell and its styles out of every /me page. Signed in without admin
  // permissions it shows the no-access state in the plain frame the signed-in layout draws.
  import { quiet } from '$lib/quiet';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import AdminShell from '$lib/admin/shell/AdminShell.svelte';
  import { adminRail as adminRailFor } from '$lib/admin/shell/sections';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import State from '$lib/components/State.svelte';
  import type { RailGroup } from '$lib/shell/rail';
  import { loadShopCounts, shopCounts } from '$lib/shop/counts.svelte';
  import { can, isAdmin, session } from '$lib/tool/session.svelte';

  let { children } = $props();

  const admin = $derived(isAdmin(session.me));

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
    const c = { review: counts?.reviewQueue?.total, reports: counts?.reportQueue?.total, items: shopCounts.items, orders: shopCounts.orders, codes: shopCounts.codes };
    return adminRailFor(can, c, page.url.pathname);
  });
</script>

{#if admin || !session.me}
  <!-- Before the account is known the frame is already there (the path decides), and its rows fill in. -->
  <AdminShell groups={adminRail} me={session.me?.id}>
    <div class="ui-page adm-page" class:pending={!session.me}>
      {#if !session.me}
        <PageTitle title="Loading - cyex.tv" />
        <State kind="loading" title="Loading your account…" />
      {:else}
        {@render children()}
      {/if}
    </div>
  </AdminShell>
{:else}
  <State level={1} role="status" title="No access here.">
    Your account has no moderation or admin permissions.
    {#snippet actions()}<a class="ui-btn secondary" href="/me">Back to your emotes</a>{/snippet}
  </State>
{/if}

<style>
  /* Its own block formatting context: the Split's pull-up margin stays inside, so the page's box
     doesn't move when the loading line gives way to the page. */
  .adm-page { display: flow-root; }
</style>
