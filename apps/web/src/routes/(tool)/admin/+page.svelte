<script lang="ts">
  import { gql } from '$lib/api';
  import { count } from '$lib/format';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
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

  type Card = { href: string; title: string; text: string; icon: IconName; figure?: number; show: boolean };
  const cards = $derived(
    [
      { href: '/admin/review', title: 'Review queue', icon: 'inbox' as const, text: 'Pending uploads, oldest first.', figure: counts?.reviewQueue?.total, show: review },
      { href: '/admin/reports', title: 'Reports', icon: 'flag' as const, text: 'Open reports. Dismiss them or take the emote down.', figure: counts?.reportQueue?.total, show: reports },
      { href: '/admin/users', title: 'Users', icon: 'users' as const, text: 'Find someone by login; block uploads or sign-in; give roles.', show: can('users.view') || can('users.restrict') || can('roles.manage') },
      { href: '/admin/roles', title: 'Roles', icon: 'key' as const, text: 'Who may do what. You manage roles below your own.', show: can('roles.manage') },
      { href: '/admin/audit', title: 'Audit log', icon: 'scroll' as const, text: 'Every audited change, newest first.', show: can('audit.view') },
      { href: counts ? `/me/sets/${counts.globalSet.id}` : '/admin', title: 'Global set', icon: 'layers' as const, text: 'Shown in every channel that uses cyex.tv.', show: can('global_set.edit') },
    ].filter((c) => c.show),
  );
</script>

<svelte:head><title>Admin - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Overview</h1><p>What your roles let you do.</p></div></div>

<ul class="cards">
  {#each cards as c (c.href)}
    <li>
      <a class="card" href={c.href}>
        <span class="icon" aria-hidden="true"><Icon name={c.icon} size={18} /></span>
        <span class="text">
          <strong>{c.title}</strong>
          <span class="ui-hint">{c.text}</span>
        </span>
        {#if c.figure !== undefined}<span class="ui-mono figure" class:some={c.figure > 0}>{count(c.figure)}</span>{/if}
      </a>
    </li>
  {/each}
</ul>

<style>
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
