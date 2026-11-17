<script lang="ts">
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import type { AdminCountsQuery } from '$lib/gql/graphql';
  import { ADMIN_COUNTS } from '$lib/queries';
  import { can } from '$lib/tool/session.svelte';
  import { fail } from '$lib/tool/notices.svelte';

  let counts = $state<AdminCountsQuery | null>(null);
  const review = can('emotes.review');
  const reports = review || can('emotes.takedown');

  $effect(() => {
    gql(ADMIN_COUNTS, { review, reports })
      .then((r) => (counts = r))
      .catch(fail);
  });

  const cards = $derived(
    [
      { href: '/admin/review', title: 'Review queue', text: 'Pending uploads, oldest first.', figure: counts?.reviewQueue?.total, show: review },
      { href: '/admin/reports', title: 'Reports', text: 'Open reports. Dismiss them or take the emote down.', figure: counts?.reportQueue?.total, show: reports },
      { href: '/admin/users', title: 'Users', text: 'Find someone by login; block uploads or sign-in; give roles.', show: can('users.view') || can('users.restrict') || can('roles.manage') },
      { href: '/admin/roles', title: 'Roles', text: 'Who may do what. You manage roles below your own.', show: can('roles.manage') },
      { href: '/admin/audit', title: 'Audit log', text: 'Every audited change, newest first.', show: can('audit.view') },
      { href: counts ? `/me/sets/${counts.globalSet.id}` : '/admin', title: 'Global set', text: 'Shown in every channel that uses cyex.tv.', show: can('global_set.edit') },
    ].filter((c) => c.show),
  );
</script>

<svelte:head><title>Admin - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Admin</h1><p>What your roles let you do.</p></div></div>

<ul class="cards">
  {#each cards as c}
    <li>
      <a class="ui-card card" href={c.href}>
        <span class="top">
          <strong>{c.title}</strong>
          {#if c.figure !== undefined}<span class="ui-mono figure">{count(c.figure)}</span>{/if}
        </span>
        <span class="ui-hint">{c.text}</span>
      </a>
    </li>
  {/each}
</ul>

<style>
  .cards { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; }
  .card { display: grid; gap: 6px; height: 100%; color: var(--text); text-decoration: none; transition: background-color var(--fast), border-color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .card:hover { background: var(--surface-hover); border-color: var(--border-strong); } }
  .card:active { opacity: .6; transition: none; }
  .top { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
  .figure { font-size: 1.1rem; font-weight: 600; }
</style>
