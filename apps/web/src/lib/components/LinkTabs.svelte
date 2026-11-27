<script lang="ts">
  // Page tabs that are links (each its own address, so the URL holds the tab): words on a hairline,
  // the current one bold, never a bar or a fill (owner rule). The profile's section tabs; the
  // /me pages use the same row. Arrow keys move between them; the row scrolls sideways on a phone.
  // A tab with an `icon` leads with it: icon and words on wide screens, the icon alone under 720px,
  // where the label turns into a tooltip (the same text is the link's aria-label either way).
  //   <LinkTabs label="lumen_fox's profile" items={[{ href, label, icon, count, current }]} />
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import { arrowNav } from '$lib/shell/rail';

  type Tab = { href: string; label: string; icon?: IconName; count?: string | number; current?: boolean };
  let { label, items }: { label: string; items: Tab[] } = $props();

  // On a phone the row scrolls: keep the current tab in view (the owner's Account and Editors sit
  // at its end). Only the row scrolls sideways, never the page.
  let row = $state<HTMLElement>();
  $effect(() => {
    const at = items.findIndex((t) => t.current);
    const el = row;
    if (!el || at < 0) return;
    const a = el.children[at] as HTMLElement | undefined;
    if (!a || el.scrollWidth <= el.clientWidth) return;
    const left = a.offsetLeft - el.offsetLeft;
    if (left < el.scrollLeft || left + a.offsetWidth > el.scrollLeft + el.clientWidth) el.scrollLeft = Math.max(0, left - 16);
  });

  // Icon-only tabs (a phone) name themselves in a tooltip. It sits outside the scrolling row, so the
  // row never clips it; shown on hover (after the tooltip delay) and at once on keyboard focus.
  let tip = $state<{ text: string; x: number; y: number } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  function showTip(ev: Event) {
    clearTimeout(timer);
    const a = (ev.target as Element | null)?.closest?.('a.icon');
    // Only a tab whose words are hidden (a phone, or a narrow row) needs the tooltip.
    const words = a instanceof HTMLElement ? a.querySelector('.tl') : null;
    if (!(a instanceof HTMLElement) || !words || getComputedStyle(words).display !== 'none') return (tip = null);
    const r = a.getBoundingClientRect();
    const text = a.querySelector('.tl')?.textContent ?? '';
    const put = () => (tip = { text, x: Math.min(Math.max(r.left + r.width / 2, 40), document.documentElement.clientWidth - 40), y: r.bottom + 6 });
    if (ev.type === 'focusin') put();
    else timer = setTimeout(put, 300);
  }
  function hideTip() {
    clearTimeout(timer);
    tip = null;
  }
</script>

<div class="row">
<nav class="tabs" aria-label={label} use:arrowNav={'horizontal'} bind:this={row} onpointerover={showTip} onfocusin={showTip} onpointerout={hideTip} onfocusout={hideTip}>
  {#each items as t (t.href)}
    <a href={t.href} class:icon={!!t.icon} aria-current={t.current ? 'page' : undefined} aria-label={t.icon ? (t.count !== undefined ? `${t.label}, ${t.count}` : t.label) : undefined}>
      {#if t.icon}<Icon name={t.icon} size={18} />{/if}
      <span class="tl" data-t={t.label} aria-hidden={t.icon ? 'true' : undefined}>{t.label}</span>{#if t.count !== undefined}<span class="n" aria-hidden={t.icon ? 'true' : undefined}>{t.count}</span>{/if}
    </a>
  {/each}
</nav>
</div>
{#if tip}<span class="tip" aria-hidden="true" style:left="{tip.x}px" style:top="{tip.y}px">{tip.text}</span>{/if}

<style>
  /* The row is a size container: when it is narrow, only the current tab keeps its words. */
  .row { container-type: inline-size; }
  .tabs { display: flex; gap: 4px; overflow-x: auto; scrollbar-width: none; border-bottom: 1px solid var(--border); margin-inline: -10px; }
  .tabs a { position: relative; flex: none; display: inline-flex; align-items: center; gap: 6px; height: 44px; padding: 0 10px; color: var(--muted);
    font-size: var(--t-ui); font-weight: 500; white-space: nowrap; text-decoration: none; transition: color var(--fast); }
  .tabs a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .tabs a:focus-visible { outline-offset: -2px; }
  /* The bold width is reserved, so the row never shifts when the current tab changes. */
  .tl { display: inline-grid; }
  .tl::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  .n { font-size: var(--t-small); color: var(--dim); font-weight: 500; font-variant-numeric: tabular-nums; }
  @media (hover: hover) and (pointer: fine) { .tabs a:hover { color: var(--text); } }

  @container (max-width: 1000px) {
    .tabs a.icon:not([aria-current='page']) { justify-content: center; min-width: 40px; padding: 0 6px; }
    .tabs a.icon:not([aria-current='page']) .tl, .tabs a.icon:not([aria-current='page']) .n { display: none; }
  }
  /* Icon tabs: icon first, then the words. Under 720px the words go and the icon is the tab (44px wide to hit). */
  @media (max-width: 719.98px) {
    .tabs a.icon { justify-content: center; min-width: 40px; padding: 0 6px; }
    .tabs a.icon .tl, .tabs a.icon .n { display: none; }
    /* No bold word to mark the current tab: its icon takes the accent (selected state, never a bar). */
    .tabs a.icon[aria-current='page'] { color: var(--accent-text); }
  }
  .tip { position: fixed; z-index: var(--z-tip); transform: translateX(-50%); padding: 5px 8px; border-radius: var(--radius-btn);
    background: var(--text); color: var(--bg); font-size: var(--t-fine); font-weight: 500; line-height: 1.2; white-space: nowrap; pointer-events: none; }
</style>
