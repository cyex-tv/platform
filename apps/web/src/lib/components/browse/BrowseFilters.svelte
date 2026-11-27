<script lang="ts">
  // The browse page's filters (in the toolbar's Filters popover, a sheet on phones):
  // collapsible sections Search, Match, Type, Flags and Tags. One model: Type is one choice (All,
  // Animated, Still); Flags are toggle chips that each narrow to emotes with that flag; Tags is just a
  // tag field (Enter adds a chip), with no suggestions. Every
  // choice applies at once (no Apply button), is the API's filter, and lands in the URL. Sections
  // start open; a closed one stays closed on this device.
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import { cleanTag, MAX_TAGS, type Browse, type Flag, type Kind } from './params';

  type Props = {
    view: Browse;
    onchange: (over: Partial<Browse>) => void;
    /** The search field (the page owns it); left out on phones, where it sits in the sticky Filters bar. */
    search?: Snippet;
    /** The emotes didn't load: the Tags section shows only when a tag is already chosen. */
    failed?: boolean;
  };
  let { view, onchange, search, failed = false }: Props = $props();

  const kinds: { value: Kind; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'animated', label: 'Animated' },
    { value: 'still', label: 'Still' },
  ];
  const flags: { key: Flag; label: string; icon: IconName; hint: string }[] = [
    { key: 'zeroWidth', label: 'Zero-width', icon: 'layers', hint: 'Drawn over the emote before it' },
    { key: 'wide', label: 'Wide', icon: 'wide', hint: 'About twice as wide as tall or more' },
  ];

  type Section = 'search' | 'match' | 'type' | 'flags' | 'tags';
  const KEY = 'cyex.browse.closed';
  let closed = $state<Section[]>([]);
  $effect(() => {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
      if (Array.isArray(v)) closed = v.filter((s) => ['search', 'match', 'type', 'flags', 'tags'].includes(s));
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

{#snippet matchBody()}
  <div class="opts">
    <label class="opt check" class:on={view.exact}>
      <input type="checkbox" checked={view.exact} onchange={(e) => onchange({ exact: e.currentTarget.checked })} />
      <span class="box" aria-hidden="true"><Icon name="check" size={12} /></span>
      <span class="lbl"><span>Exact name</span><small>The name equals what you typed</small></span>
    </label>
  </div>
{/snippet}

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
  <div class="tags flags" role="group" aria-label="Flags">
    {#each flags as f (f.key)}
      <button class="tag flag" type="button" title={f.hint} aria-pressed={view[f.key]} aria-describedby="flag-{f.key}" onclick={() => onchange({ [f.key]: !view[f.key] })}>
        <span class="fi" aria-hidden="true"><Icon name={view[f.key] ? 'check' : f.icon} size={14} /></span>{f.label}
      </button>
      <span class="sr-only" id="flag-{f.key}">{f.hint}</span>
    {/each}
  </div>
{/snippet}

{#snippet tagsBody()}
  <div class="tagbody">
    <TagInput
      tags={view.tags}
      onchange={(t) => onchange({ tags: t })}
      label="Tags the emotes must have"
      placeholder="Type a tag, press Enter"
      max={MAX_TAGS}
      clean={cleanTag}
    />
  </div>
{/snippet}

<div class="filters">
  {#if search}{@render section('search', 'Search', searchBody)}{/if}
  {#if view.q}{@render section('match', 'Match', matchBody)}{/if}
  {@render section('type', 'Type', typeBody)}
  {@render section('flags', 'Flags', flagsBody)}
  {#if !failed || view.tags.length}{@render section('tags', 'Tags', tagsBody)}{/if}
</div>

<style>
  .filters { display: grid; min-width: 0; }
  .sec, .body { min-width: 0; }
  .sec + .sec { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  /* The summary reads like the rail's group caption, with a chevron that turns when closed. */
  summary { display: flex; align-items: center; justify-content: space-between; min-height: 40px; padding: 0 10px;
    border-radius: var(--radius); list-style: none; cursor: pointer; font-size: var(--t-label); font-weight: 600;
    letter-spacing: .06em; text-transform: uppercase; color: var(--dim); transition: color var(--fast); }
  summary::-webkit-details-marker { display: none; }
  summary :global(svg) { transition: rotate var(--base) var(--ease); }
  details:not([open]) summary :global(svg) { rotate: -90deg; }
  summary:focus-visible { outline-offset: -2px; }
  .body { padding-top: 2px; }

  .opts { display: grid; gap: 1px; }
  .opt { position: relative; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 6px 10px;
    border-radius: var(--radius); font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer;
    transition: background-color var(--fast), color var(--fast); }
  .opt input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .opt:has(input:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .check { align-items: flex-start; }
  .check .box { margin-top: 3px; }
  .opt .lbl { flex: 1; min-width: 0; display: grid; line-height: 1.35; }
  .opt small { font-size: 12px; font-weight: 400; color: var(--dim); }
  .tick { display: inline-flex; color: var(--accent-text); opacity: 0; }
  .opt:not(.check).on { color: var(--text); font-weight: 600; }
  .opt.on .tick { opacity: 1; }
  .opt.check.on { color: var(--text); font-weight: 600; }
  .opt.check.on small { font-weight: 400; }
  .box { flex: none; display: inline-grid; place-items: center; width: 16px; height: 16px; border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm); color: transparent; background: var(--bg);
    transition: background-color var(--fast), border-color var(--fast); }
  .on .box { border-color: var(--accent-text); color: var(--accent-text); } /* the tick is the accent icon; no fill */
  .opt:active { opacity: .6; transition: none; }

  .tagbody { display: grid; gap: 8px; padding: 2px; min-width: 0; }
  .tags { display: flex; flex-wrap: wrap; gap: 8px 6px; padding: 4px 2px; min-width: 0; }
  .flag { gap: 6px; padding: 0 10px 0 8px; }
  .fi { display: inline-flex; color: var(--dim); }
  .flag[aria-pressed='true'] .fi { color: var(--accent-text); }
  /* 32px to see, 40px to hit (the row gap takes the rest). Long tags end in an ellipsis. */
  .tag { position: relative; display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 100%; height: 32px; padding: 0 8px;
    border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); color: var(--muted);
    font: inherit; font-size: 13px; font-weight: 500; cursor: pointer;
    transition: background-color var(--fast), border-color var(--fast), color var(--fast); }
  .tag::after { content: ''; position: absolute; inset: -4px 0; }
  .tag[aria-pressed='true'] { color: var(--text); font-weight: 700; } /* bold, no fill or accent border */
  .tag:active { opacity: .6; transition: none; }
  .tag:focus-visible { outline-offset: 1px; }
  /* High contrast drops bold, the tick colour and the border cue: the selected option keeps a system-colour outline. */
  @media (forced-colors: active) {
    .tag[aria-pressed='true'] { outline: 1px solid Highlight; outline-offset: -2px; }
    .opt.on:not(:has(input:focus-visible)) { outline: 1px solid Highlight; outline-offset: -2px; }
    .on .box { border-color: Highlight; }
  }

  @media (pointer: coarse) { .opt { min-height: 44px; } .tag { height: 36px; } .tags { row-gap: 8px; } summary { min-height: 44px; } }
  @media (hover: hover) and (pointer: fine) {
    summary:hover { color: var(--text); }
    .opt:hover { color: var(--text); }
    .opt:not(.check).on:hover { color: var(--text); }
    .tag:hover { border-color: var(--border-strong); color: var(--text); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) summary :global(svg) { transition: none; } }
</style>
