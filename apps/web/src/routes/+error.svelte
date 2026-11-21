<script lang="ts">
  // Every error page (404, 403, 500): the one error pattern (Problem) with its way out, then a
  // short list of places to go instead, so a dead link never ends the visit. The address that
  // failed is shown, so a typo is easy to spot.
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { causeOfStatus, problemTitle } from '$lib/components/problem';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { elsewhere } from './error-links';

  const cause = $derived(causeOfStatus(page.status));
  const links = $derived(elsewhere(page.url.pathname));
  const ICON: Record<string, IconName> = { search: 'search', shop: 'shopping-bag', help: 'scroll', changelog: 'list' };
</script>

<svelte:head>
  <title>{problemTitle(cause)}</title>
</svelte:head>

<ColumnLayout width="measure" top>
  <Problem {cause} level={1} onretry={() => invalidateAll()} />
  <div class="after">
    <p class="where"><span class="label">Address</span><code>{page.url.pathname}</code><span class="code">{page.status}</span></p>
    <h2 class="label">Try instead</h2>
    <ul class="links">
      {#each links as l (l.href)}
        <li><a href={l.href}><Icon name={ICON[l.icon]} size={16} /><span>{l.label}</span><Icon name="chevron-right" size={14} /></a></li>
      {/each}
    </ul>
  </div>
</ColumnLayout>

<style>
  p, h2 { margin: 0; }
  .after { display: grid; gap: 4px; margin: 20px 0 48px; }
  .label { display: flex; align-items: center; min-height: 36px; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .where { display: flex; align-items: center; gap: 12px; min-width: 0; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
  .where .label { flex: none; }
  .where code { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--font-mono); font-size: var(--t-small); color: var(--text); }
  .where .code { flex: none; font-size: var(--t-small); color: var(--dim); font-variant-numeric: tabular-nums; }
  .links { list-style: none; margin: 0; padding: 0; display: grid; }
  .links li + li { border-top: 1px solid var(--border); }
  .links a { display: flex; align-items: center; gap: 10px; min-height: 44px; margin: 0 -6px; padding: 0 6px; border-radius: var(--radius);
    color: var(--text); font-size: var(--t-small); font-weight: 600; transition: background-color var(--fast); }
  .links a span { flex: 1; min-width: 0; }
  .links a :global(svg) { flex: none; color: var(--dim); }
  .links a:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .links a:hover { background: var(--row-hover); } }
</style>
