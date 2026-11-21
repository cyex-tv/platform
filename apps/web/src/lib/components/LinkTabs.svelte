<script lang="ts">
  // Page tabs that are links (each its own address, so the URL holds the tab): words on a hairline,
  // the current one bold, never a bar or a fill (owner rule). The profile's section tabs; the
  // /me pages use the same row. Arrow keys move between them; the row scrolls sideways on a phone.
  //   <LinkTabs label="lumen_fox's profile" items={[{ href, label, count, current }]} />
  import { arrowNav } from '$lib/shell/rail';

  type Tab = { href: string; label: string; count?: string | number; current?: boolean };
  let { label, items }: { label: string; items: Tab[] } = $props();
</script>

<nav class="tabs" aria-label={label} use:arrowNav={'horizontal'}>
  {#each items as t (t.href)}
    <a href={t.href} aria-current={t.current ? 'page' : undefined}>
      <span class="tl" data-t={t.label}>{t.label}</span>{#if t.count !== undefined}<span class="n">{t.count}</span>{/if}
    </a>
  {/each}
</nav>

<style>
  .tabs { display: flex; gap: 4px; overflow-x: auto; scrollbar-width: none; border-bottom: 1px solid var(--border); margin-inline: -10px; }
  .tabs a { flex: none; display: inline-flex; align-items: center; gap: 6px; height: 44px; padding: 0 10px; color: var(--muted);
    font-size: var(--t-ui); font-weight: 500; white-space: nowrap; text-decoration: none; transition: color var(--fast); }
  .tabs a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .tabs a:focus-visible { outline-offset: -2px; }
  /* The bold width is reserved, so the row never shifts when the current tab changes. */
  .tl { display: inline-grid; }
  .tl::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  .n { font-size: var(--t-small); color: var(--dim); font-weight: 500; font-variant-numeric: tabular-nums; }
  @media (hover: hover) and (pointer: fine) { .tabs a:hover { color: var(--text); } }
</style>
