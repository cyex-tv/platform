<script lang="ts">
  // Rows that each lead somewhere (related pages, downloads, help): an optional icon, the label
  // with an optional line under it, and a chevron (an arrow out for another site). 44px rows with
  // a hairline between them; the whole row is the link.
  //   <LinkRows label="Related" items={[{ href: '/help', label: 'Help', hint: 'How cyex.tv works', icon: 'book' }]} />
  import Icon, { type IconName } from './Icon.svelte';

  type Row = { href: string; label: string; hint?: string; icon?: IconName; external?: boolean };
  let { items, label }: { items: Row[]; label?: string } = $props();
</script>

<ul class="rows" aria-label={label}>
  {#each items as r (r.href)}
    <li>
      <a href={r.href} target={r.external ? '_blank' : undefined} rel={r.external ? 'noopener' : undefined}>
        {#if r.icon}<Icon name={r.icon} size={16} />{/if}
        <span class="txt"><span class="lb">{r.label}</span>{#if r.hint}<span class="hint">{r.hint}</span>{/if}</span>
        {#if r.external}<span class="sr-only"> (opens in a new tab)</span>{/if}
        <span class="go" aria-hidden="true"><Icon name={r.external ? 'arrow-up-right' : 'chevron-right'} size={14} /></span>
      </a>
    </li>
  {/each}
</ul>

<style>
  .rows { list-style: none; margin: 0; padding: 0; display: grid; }
  li + li { border-top: 1px solid var(--border); }
  a { display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 6px 0; color: var(--text); text-decoration: none; }
  a > :global(svg) { flex: none; color: var(--dim); }
  .txt { flex: 1; display: grid; min-width: 0; line-height: 1.35; }
  .lb { font-size: var(--t-small); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { font-size: var(--t-fine); color: var(--muted); }
  .go { display: inline-flex; flex: none; color: var(--dim); transition: translate var(--fast) var(--ease); }
  a:focus-visible { outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) {
    a:hover .lb { text-decoration: underline; text-underline-offset: 3px; }
    a:hover .go { color: var(--text); translate: 2px 0; }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .go { transition: none; } }
</style>
