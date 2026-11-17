<script lang="ts">
  import { page } from '$app/state';
  import { ADMIN_PERMISSIONS, can, session } from '$lib/tool/session.svelte';

  let { children } = $props();

  const sections = $derived(
    [
      { href: '/admin', label: 'Overview', show: true },
      { href: '/admin/review', label: 'Review', show: can('emotes.review') },
      { href: '/admin/reports', label: 'Reports', show: can('emotes.review') || can('emotes.takedown') },
      { href: '/admin/users', label: 'Users', show: can('users.view') || can('users.restrict') || can('roles.manage') },
      { href: '/admin/roles', label: 'Roles', show: can('roles.manage') },
      { href: '/admin/audit', label: 'Audit log', show: can('audit.view') },
    ].filter((s) => s.show),
  );
  const allowed = $derived(session.me?.permissions.some((p) => ADMIN_PERMISSIONS.includes(p)) ?? false);
</script>

{#if !allowed}
  <div class="ui-empty">
    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="11" width="14" height="10" rx="1" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
    <p>Your account has no moderation or admin permissions.</p>
    <a class="ui-btn secondary" href="/me">Back to your emotes</a>
  </div>
{:else}
  <nav class="ui-seg sub" aria-label="Admin">
    {#each sections as s}
      <a href={s.href} aria-current={page.url.pathname === s.href ? 'page' : undefined}>{s.label}</a>
    {/each}
  </nav>
  {@render children()}
{/if}

<style>
  .sub { margin-bottom: 20px; }
</style>
