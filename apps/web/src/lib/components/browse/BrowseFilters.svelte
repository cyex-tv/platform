<script lang="ts">
  // The browse page's filter rail content (inside SideRail, or its Filters sheet on phones):
  // collapsible sections Search, Type, Flags and Tags. Every choice applies at once (no Apply
  // button) and lands in the URL. Sections start open; a closed one stays closed on this device.
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { count as fmt } from '$lib/format';
  import type { Browse, Kind } from './params';

  type Props = {
    view: Browse;
    onchange: (over: Partial<Browse>) => void;
    /** The search field (the page owns it); left out on phones, where it sits in the sticky Filters bar. */
    search?: Snippet;
    /** The most used tags in what is loaded. */
    tags: { tag: string; count: number }[];
  };
  let { view, onchange, search, tags }: Props = $props();

  const kinds: { value: Kind; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'animated', label: 'Animated' },
    { value: 'still', label: 'Still' },
  ];
  const flags: { key: 'zeroWidth' | 'wide' | 'nsfw'; label: string; hint: string }[] = [
    { key: 'zeroWidth', label: 'Zero-width', hint: 'Stack on the emote before them' },
    { key: 'wide', label: 'Wide', hint: 'About twice as wide as tall' },
    { key: 'nsfw', label: 'Show NSFW', hint: 'Hidden unless this is on' },
  ];
  // The selected tag stays listed even when the loaded emotes no longer count it.
  const shownTags = $derived(view.tag && !tags.some((t) => t.tag === view.tag) ? [{ tag: view.tag, count: 0 }, ...tags] : tags);

  type Section = 'search' | 'type' | 'flags' | 'tags';
  const KEY = 'cyex.browse.closed';
  let closed = $state<Section[]>([]);
  $effect(() => {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
      if (Array.isArray(v)) closed = v.filter((s) => ['search', 'type', 'flags', 'tags'].includes(s));
    } catch {
      /* private mode or junk: all open */
    }
  });
  function toggle(s: Section, e: Event) {
    const open = (e.currentTarget as HTMLDetailsElement).open;
    const next = open ? closed.filter((c) => c !== s) : [...new Set([...closed, s])];
    if (next.length === closed.length && next.every((c) => closed.includes(c))) return;
    closed = next;
    try {
      localStorage.setItem(KEY, JSON.stringify(closed));
    } catch {
      /* not remembered */
    }
  }
</script>

{#snippet section(id: Section, title: string, body: Snippet)}
  <details class="sec" open={!closed.includes(id)} ontoggle={(e) => toggle(id, e)}>
    <summary><span>{title}</span><Icon name="chevron-down" size={16} /></summary>
    <div class="body">{@render body()}</div>
  </details>
{/snippet}

{#snippet searchBody()}{@render search?.()}{/snippet}

{#snippet typeBody()}
  <div class="opts" role="radiogroup" aria-label="Type">
    {#each kinds as k (k.value)}
      <label class="opt" class:on={view.kind === k.value}>
        <input type="radio" name="kind" value={k.value} checked={view.kind === k.value} onchange={() => onchange({ kind: k.value })} />
        <span class="lbl">{k.label}</span>
        <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
      </label>
    {/each}
  </div>
{/snippet}

{#snippet flagsBody()}
  <div class="opts">
    {#each flags as f (f.key)}
      <label class="opt check" class:on={view[f.key]}>
        <input type="checkbox" checked={view[f.key]} onchange={(e) => onchange({ [f.key]: e.currentTarget.checked })} />
        <span class="box" aria-hidden="true"><Icon name="check" size={12} /></span>
        <span class="lbl"><span>{f.label}</span><small>{f.hint}</small></span>
      </label>
    {/each}
  </div>
{/snippet}

{#snippet tagsBody()}
  {#if shownTags.length}
    <div class="tags" role="group" aria-label="Tags">
      {#each shownTags as t (t.tag)}
        <button class="tag" type="button" aria-pressed={view.tag === t.tag} onclick={() => onchange({ tag: view.tag === t.tag ? '' : t.tag })}>
          <bdi>#{t.tag}</bdi>{#if t.count}<span class="n">{fmt(t.count)}</span>{/if}
        </button>
      {/each}
    </div>
  {:else}
    <p class="none">No tags on these emotes.</p>
  {/if}
{/snippet}

<div class="filters">
  {#if search}{@render section('search', 'Search', searchBody)}{/if}
  {@render section('type', 'Type', typeBody)}
  {@render section('flags', 'Flags', flagsBody)}
  {@render section('tags', 'Tags', tagsBody)}
</div>

<style>
  .filters { display: grid; }
  .sec + .sec { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  /* The summary reads like the rail's group caption, with a chevron that turns when closed. */
  summary { display: flex; align-items: center; justify-content: space-between; min-height: 28px; padding: 0 10px;
    border-radius: var(--radius); list-style: none; cursor: pointer; font-size: var(--t-label); font-weight: 600;
    letter-spacing: .06em; text-transform: uppercase; color: var(--dim); transition: color var(--fast); }
  summary::-webkit-details-marker { display: none; }
  summary :global(svg) { transition: rotate var(--base) var(--ease); }
  details:not([open]) summary :global(svg) { rotate: -90deg; }
  summary:focus-visible { outline-offset: -2px; }
  .body { padding-top: 6px; }

  .opts { display: grid; gap: 1px; }
  .opt { position: relative; display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 6px 10px;
    border-radius: var(--radius); font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer;
    transition: background-color var(--fast), color var(--fast); }
  .opt input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .opt:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .check { align-items: flex-start; }
  .check .box { margin-top: 3px; }
  .opt .lbl { flex: 1; min-width: 0; display: grid; line-height: 1.35; }
  .opt small { font-size: 12px; font-weight: 400; color: var(--dim); }
  .tick { display: inline-flex; color: var(--accent-text); opacity: 0; }
  .opt:not(.check).on { background: var(--accent-dim); color: var(--accent-text); box-shadow: inset 2px 0 0 var(--accent); }
  .opt.on .tick { opacity: 1; }
  .opt.check.on { color: var(--text); }
  .box { flex: none; display: inline-grid; place-items: center; width: 16px; height: 16px; border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm); color: transparent; background: var(--bg);
    transition: background-color var(--fast), border-color var(--fast); }
  .on .box { background: var(--accent); border-color: var(--accent); color: var(--accent-ink); }
  .opt:active { opacity: .6; transition: none; }

  .tags { display: flex; flex-wrap: wrap; gap: 6px; padding: 0 2px; }
  .tag { display: inline-flex; align-items: center; gap: 6px; max-width: 100%; height: 28px; padding: 0 8px;
    border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); color: var(--muted);
    font: inherit; font-size: 13px; font-weight: 500; cursor: pointer;
    transition: background-color var(--fast), border-color var(--fast), color var(--fast); }
  .tag bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tag .n { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .tag[aria-pressed='true'] { background: var(--accent-dim); border-color: var(--accent-border); color: var(--accent-text); }
  .tag[aria-pressed='true'] .n { color: currentColor; }
  .tag:active { opacity: .6; transition: none; }
  .tag:focus-visible { outline-offset: 1px; }
  .none { margin: 0; padding: 0 10px; font-size: var(--t-fine); color: var(--dim); }

  @media (pointer: coarse) { .opt { min-height: 44px; } .tag { height: 36px; } summary { min-height: 40px; } }
  @media (hover: hover) and (pointer: fine) {
    summary:hover { color: var(--text); }
    .opt:hover { background: var(--row-hover); color: var(--text); }
    .opt:not(.check).on:hover { background: var(--accent-dim); color: var(--accent-text); }
    .tag:hover { border-color: var(--border-strong); color: var(--text); }
    .tag[aria-pressed='true']:hover { color: var(--accent-text); }
  }
  @media (forced-colors: active) {
    .opt.on, .tag[aria-pressed='true'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
  @media (prefers-reduced-motion: reduce) { summary :global(svg) { transition: none; } }
</style>
