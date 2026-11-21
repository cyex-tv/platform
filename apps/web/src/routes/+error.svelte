<script lang="ts">
  // Every error page (404, 403, 500), flat and centred: the status in mono, the one error pattern
  // (Problem, bare) with its way out, then a
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

<ColumnLayout top>
  <div class="center">
    <div class="flat">
      <p class="code-line"><span class="label">Error</span><span class="status">{page.status}</span></p>
      <div class="problem"><Problem {cause} level={1} bare onretry={() => invalidateAll()} /></div>
      <dl class="where">
        <dt class="label">Address</dt><dd><code>{page.url.pathname}</code></dd>
      </dl>
      <h2 class="label">Try instead</h2>
      <ul class="links">
        {#each links as l (l.href)}
          <li><a href={l.href}><Icon name={ICON[l.icon]} size={16} /><span>{l.label}</span><Icon name="chevron-right" size={14} /></a></li>
        {/each}
      </ul>
    </div>
  </div>
</ColumnLayout>

<style>
  p, h2, dl, dd { margin: 0; }
  /* Flat and centred, like sign-in: no card; the status in mono, uppercase labels over hairline rows. */
  .center { display: grid; justify-items: center; padding-block: clamp(16px, 5vw, 56px) 48px; }
  .flat { width: 100%; max-width: var(--measure-form); }
  .label { display: flex; align-items: center; min-height: 40px; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .code-line { display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border); }
  .status { font-family: var(--font-mono); font-size: var(--t-small); font-weight: 600; color: var(--text); font-variant-numeric: tabular-nums; }
  .problem :global(.state) { justify-items: start; text-align: left; min-height: 0; padding: 24px 0 28px; }
  .problem :global(.state > svg) { display: none; }
  .problem :global(.actions) { justify-content: flex-start; }
  .where { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: center; gap: 16px; border-block: 1px solid var(--border); }
  .where code { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; font-family: var(--font-mono); font-size: var(--t-small); color: var(--text); }
  h2.label { margin-top: 20px; border-bottom: 1px solid var(--border); }
  .links { list-style: none; margin: 0; padding: 0; display: grid; }
  .links li { border-bottom: 1px solid var(--border); }
  .links a { display: flex; align-items: center; gap: 10px; min-height: 48px; margin: 0 -6px; padding: 0 6px; border-radius: var(--radius);
    color: var(--text); font-size: var(--t-small); font-weight: 600; transition: background-color var(--fast); }
  .links a span { flex: 1; min-width: 0; }
  .links a :global(svg) { flex: none; color: var(--dim); }
  .links a:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .links a:hover { background: var(--row-hover); } }
</style>
